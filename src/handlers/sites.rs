use std::io;

use crate::db::DbPool;
use crate::models::{File, Site};
use crate::services::s3;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{body::SizedStream, http::Method, web, HttpResponse, Responder};
use chrono::Utc;
use diesel::prelude::*;
use futures_util::{stream, StreamExt};
use serde_json::json;
use tokio_util::io::ReaderStream;

#[derive(MultipartForm)]
pub struct CreateSiteForm {
    #[multipart(rename = "subdomain")]
    subdomain: Text<String>,
    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

pub async fn create_site(
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
    MultipartForm(form): MultipartForm<CreateSiteForm>,
) -> impl Responder {
    // purpose of using `use crate::schema::files::dsl::*;` and `use crate::schema::sites::dsl::*;
    // is to avoid writing the full path of the schema in the query
    // for example, instead of writing `crate::schema::files::dsl::files` we can just write `files`
    use crate::schema::files::dsl::*;
    use crate::schema::sites::dsl::*;

    let uploading_files = form.files;
    for file in &uploading_files {
        let content_type = file.content_type.clone().unwrap();
        if content_type != "text/html" {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid file type. Only text/html files are allowed",
            }));
        }
    }

    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let now = Utc::now().naive_utc();
    let new_site = Site {
        id: ulid::Ulid::new().to_string(),
        subdomain: form.subdomain.clone(),
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(sites)
        .values(&new_site)
        .execute(&mut conn)
        .expect("Error saving new site");

    let site_path = format!("sites/{}/", new_site.id.clone());
    let uploaded_files = &s3_client
        .upload_files(uploading_files, &site_path)
        .await
        .expect("Error uploading files");

    println!("Uploaded files: {:?}", uploaded_files);

    let new_files: Vec<File> = uploaded_files
        .into_iter()
        .map(|file| {
            let now = Utc::now().naive_utc();
            let file_name = file.filename.clone();
            let file_path = format!("{}{}", site_path, file_name);
            let file_mime_type = file.content_type.clone();
            File {
                id: ulid::Ulid::new().to_string(),
                site_id: new_site.id.clone(),
                name: file_name,
                path: file_path,
                mime_type: file_mime_type,
                size: file.size,
                is_index: true,
                created_at: now,
                updated_at: now,
            }
        })
        .collect();

    diesel::insert_into(files)
        .values(&new_files)
        .execute(&mut conn)
        .expect("Error saving new files");

    HttpResponse::Ok().json(json!({
        "message": "Site created successfully",
    }))
}

pub async fn serve_site_file(
    host: web::Path<String>,
    method: Method,
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
) -> impl Responder {
    use crate::schema::files::dsl::*;
    use crate::schema::sites::dsl::*;

    let site_host = host.into_inner().replace(".nanohost.localhost", "");
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let site = match sites
        .filter(subdomain.eq(site_host))
        .first::<Site>(&mut conn)
    {
        Ok(site) => site,
        Err(_) => {
            return HttpResponse::NotFound().finish();
        }
    };

    let index_file = match files
        .filter(site_id.eq(site.id.clone()).and(is_index.eq(true)))
        .first::<File>(&mut conn)
    {
        Ok(file) => file,
        Err(_) => {
            return HttpResponse::NotFound().finish();
        }
    };

    let (file_size, file_stream) = match s3_client.fetch_file(&index_file.path).await {
        Some((file_size, file_stream)) => (file_size, file_stream),
        None => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let stream = match method {
        // data stream for GET requests
        Method::GET => ReaderStream::new(file_stream.into_async_read()).boxed_local(),

        // empty stream for HEAD requests
        Method::HEAD => stream::empty::<Result<_, io::Error>>().boxed_local(),

        _ => unreachable!(),
    };

    HttpResponse::Ok()
        .content_type(index_file.mime_type)
        .no_chunking(file_size)
        .body(SizedStream::new(file_size, stream))
}

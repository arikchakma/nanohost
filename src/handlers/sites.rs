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
    #[multipart(rename = "domain")]
    domain: Text<String>,
    #[multipart(rename = "suffix")]
    suffix: Text<String>,

    #[multipart(rename = "index_file")]
    index_file: Text<String>,

    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

pub async fn create_site(
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
    MultipartForm(form): MultipartForm<CreateSiteForm>,
) -> impl Responder {
    use crate::schema::files::dsl::*;
    use crate::schema::sites::dsl::*;

    let uploading_files = form.files;
    println!("Uploading files: {:?}", uploading_files);
    for file in &uploading_files {
        let content_type = match file.content_type.clone() {
            Some(content_type) => content_type,
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Invalid file type. Only text/html and text/css files are allowed",
                }));
            }
        };

        if content_type != "text/html" && content_type != "text/css" {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid file type. Only text/html and text/css files are allowed",
            }));
        }
    }

    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let now = Utc::now().naive_utc();
    let formatted_host = format!("{}{}", form.domain.clone(), form.suffix.clone());
    let new_site = Site {
        id: ulid::Ulid::new().to_string(),
        host: formatted_host,
        index_file: Some(form.index_file.clone()),
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
                name: file_name.clone(),
                path: file_path,
                mime_type: file_mime_type,
                size: file.size,
                is_index: file_name == form.index_file.clone(),
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
    site_path: web::Path<String>,
    method: Method,
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
) -> impl Responder {
    use crate::schema::files::dsl::*;
    use crate::schema::sites::dsl::*;

    let site_path = site_path.into_inner();
    let path_segments: Vec<&str> = site_path.split('/').collect();
    let (host_name, formatted_path) = match path_segments.split_first() {
        Some((host_name, remaining_segments)) => (host_name, remaining_segments.join("/")),
        None => {
            return HttpResponse::NotFound().finish();
        }
    };

    let is_index_page = formatted_path.clone() == "/" || formatted_path.is_empty();

    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let site: Site = match sites
        .filter(host.eq(host_name))
        .select(Site::as_select())
        .first::<Site>(&mut conn)
    {
        Ok(site) => site,
        Err(_) => {
            return HttpResponse::NotFound().finish();
        }
    };

    let site_path = format!(
        "sites/{}/{}",
        site.id,
        if is_index_page {
            site.index_file.unwrap_or("index.html".to_string())
        } else {
            formatted_path
        }
    );
    println!("Site path: {}", site_path);
    let associated_file = match files
        .filter(
            site_id
                .eq(site.id.clone())
                .and(is_index.eq(is_index_page))
                .and(path.eq(site_path)),
        )
        .first::<File>(&mut conn)
    {
        Ok(file) => file,
        Err(_) => {
            println!("File not found");
            return HttpResponse::NotFound().finish();
        }
    };

    let (file_size, file_stream) = match s3_client.fetch_file(&associated_file.path).await {
        Some((file_size, file_stream)) => (file_size, file_stream),
        None => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let stream = match method {
        Method::GET => ReaderStream::new(file_stream.into_async_read()).boxed_local(),
        Method::HEAD => stream::empty::<Result<_, io::Error>>().boxed_local(),

        _ => unreachable!(),
    };

    HttpResponse::Ok()
        .content_type(associated_file.mime_type)
        .no_chunking(file_size)
        .body(SizedStream::new(file_size, stream))
}

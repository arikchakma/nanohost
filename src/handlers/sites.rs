use crate::db::DbPool;
use crate::models::{File, Site};
use crate::services::{cloudfront_key_value, s3};
use crate::utils::zip::extract_file;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use diesel::prelude::*;
use serde_json::json;

enum SiteType {
    Html,
    Zip,
}

#[derive(MultipartForm)]
pub struct CreateSiteForm {
    domain: Text<String>,
    suffix: Text<String>,

    site_type: Text<String>,
    index_file: Text<String>,

    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

fn validate_files(site_type: SiteType, files: Vec<TempFile>) -> Result<Vec<TempFile>, String> {
    match site_type {
        SiteType::Html => {
            const MAX_FILE_SIZE: usize = 2 * 1024 * 1024; // 2MB

            for file in &files {
                let content_type = file.content_type.clone().ok_or_else(|| "")?;
                if content_type != "text/html" && content_type != "text/css" {
                    return Err(
                        "Invalid file type. Only text/html and text/css files are allowed"
                            .to_string(),
                    );
                }

                if file.size > MAX_FILE_SIZE {
                    return Err("File size is too large. Maximum size is 2MB".to_string());
                }
            }

            Ok(files)
        }
        SiteType::Zip => {
            // If the site type is zip, we will take the first file and check if it's a zip file
            // if it's not a zip file, we will return an error
            // otherwise extract the files and add them to the updated_files vector
            let first_file = match files.into_iter().next() {
                Some(file) => file,
                None => {
                    return Err("No files found".to_string());
                }
            };

            let content_type = first_file.content_type.clone().ok_or_else(|| "")?;
            if content_type != "application/zip" {
                return Err("Invalid file type. Only zip files are allowed".to_string());
            }

            const MAX_ZIP_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB
            if first_file.size > MAX_ZIP_FILE_SIZE {
                return Err("Zip file size is too large. Maximum size is 5MB".to_string());
            }

            let files = extract_file(first_file.file.into_file());
            Ok(files)
        }
    }
}

pub async fn create_site(
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
    cloudfront_key_value_client: web::Data<cloudfront_key_value::Client>,
    // dynamodb_client: web::Data<dynamodb::Client>,
    MultipartForm(form): MultipartForm<CreateSiteForm>,
) -> impl Responder {
    use crate::schema::files::dsl::*;
    use crate::schema::sites::dsl::*;

    let site_type = match form.site_type.clone().as_str() {
        "html" => SiteType::Html,
        "zip" => SiteType::Zip,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid site type. Only 'html' and 'zip' are allowed",
            }));
        }
    };

    let uploading_files = match validate_files(site_type, form.files) {
        Ok(updated_files) => updated_files,
        Err(message) => {
            return HttpResponse::BadRequest().json(json!({
                "message": message,
            }));
        }
    };

    let mut conn = pool.get().expect("couldn't get db connection from pool");

    // Check if the host is already taken
    // If it is, return an error
    let formatted_host = format!("{}{}", form.domain.clone(), form.suffix.clone());
    match sites
        .filter(host.eq(formatted_host.clone()))
        .select(Site::as_select())
        .first(&mut conn)
    {
        Ok(_) => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Domain is already taken",
            }));
        }
        Err(_) => false,
    };

    let now = Utc::now().naive_utc();
    let new_site = Site {
        id: ulid::Ulid::new().to_string(),
        host: formatted_host.clone(),
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

    let cloudfront_key = formatted_host.clone();
    let cloudfront_value = format!("{}=x={}", new_site.id.clone(), Utc::now().timestamp());
    // let mut dynamodb_values: HashMap<String, AttributeValue> = HashMap::new();
    // dynamodb_values.insert(
    //     "host".to_string(),
    //     AttributeValue::S(formatted_host.clone()),
    // );
    // dynamodb_values.insert("siteId".to_string(), AttributeValue::S(new_site.id.clone()));
    // dynamodb_values.insert(
    //     "cacheKey".to_string(),
    //     AttributeValue::S(format!(
    //         "{}=x={}",
    //         new_site.id.clone(),
    //         Utc::now().timestamp()
    //     )),
    // );
    // dynamodb_values.insert(
    //     "timestamp".to_string(),
    //     AttributeValue::N(Utc::now().timestamp().to_string()),
    // );

    // dynamodb_client
    //     .put_item(dynamodb_values)
    //     .await
    //     .expect("Error putting item");

    cloudfront_key_value_client
        .set_value(&cloudfront_key, &cloudfront_value)
        .await
        .expect("Error setting cloudfront key value");

    HttpResponse::Ok().json(json!({
        "message": format!("You can now access your site at: https://{} with site id: {}", new_site.host, new_site.id)
    }))
}

pub async fn update_site(
    path_data: web::Path<String>,
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
    cloudfront_key_value_client: web::Data<cloudfront_key_value::Client>,
    MultipartForm(form): MultipartForm<CreateSiteForm>,
) -> impl Responder {
    use crate::schema::files::dsl::{site_id as file_site_id, *};
    use crate::schema::sites::dsl::{id as site_id, *};

    let site_type = match form.site_type.clone().as_str() {
        "html" => SiteType::Html,
        "zip" => SiteType::Zip,
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid site type. Only 'html' and 'zip' are allowed",
            }));
        }
    };

    let uploading_files = match validate_files(site_type, form.files) {
        Ok(updated_files) => updated_files,
        Err(message) => {
            return HttpResponse::BadRequest().json(json!({
                "message": message,
            }));
        }
    };

    let site_id_to_update = path_data.into_inner();
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let site: Site = match sites
        .filter(site_id.eq(site_id_to_update.clone()))
        .select(Site::as_select())
        .first(&mut conn)
    {
        Ok(site) => site,
        Err(_) => {
            return HttpResponse::NotFound().finish();
        }
    };

    let site_path = format!("sites/{}/", site.id.clone());
    let uploaded_files = &s3_client
        .upload_files(uploading_files, &site_path)
        .await
        .expect("Error uploading files");

    diesel::delete(files.filter(file_site_id.eq(site.id.clone())))
        .execute(&mut conn)
        .expect("Error deleting old files");

    let new_files: Vec<File> = uploaded_files
        .into_iter()
        .map(|file| {
            let now = Utc::now().naive_utc();
            let file_name = file.filename.clone();
            let file_path = format!("{}{}", site_path, file_name);
            let file_mime_type = file.content_type.clone();
            File {
                id: ulid::Ulid::new().to_string(),
                site_id: site.id.clone(),
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

    let cloudfront_key = site.host.clone();
    let cloudfront_value = format!("{}=x={}", site.id.clone(), Utc::now().timestamp());

    cloudfront_key_value_client
        .set_value(&cloudfront_key, &cloudfront_value)
        .await
        .expect("Error setting cloudfront key value");

    HttpResponse::Ok().json(json!({
        "message": format!("Site updated successfully")
    }))
}

pub async fn delete_site(
    path_data: web::Path<String>,
    pool: web::Data<DbPool>,
    s3_client: web::Data<s3::Client>,
    cloudfront_key_value_client: web::Data<cloudfront_key_value::Client>,
) -> impl Responder {
    use crate::schema::files::dsl::{site_id as file_site_id, *};
    use crate::schema::sites::dsl::{id as site_id, *};

    let site_id_to_delete = path_data.into_inner();
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let site: Site = match sites
        .filter(site_id.eq(site_id_to_delete.clone()))
        .select(Site::as_select())
        .first(&mut conn)
    {
        Ok(site) => site,
        Err(_) => {
            return HttpResponse::NotFound().finish();
        }
    };

    let file_paths: Vec<String> = files
        .filter(file_site_id.eq(site.id.clone()))
        .select(path)
        .load::<String>(&mut conn)
        .expect("Error loading files");

    match s3_client.delete_files(file_paths).await {
        true => (),
        false => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    cloudfront_key_value_client
        .delete_value(&site.host)
        .await
        .expect("Error deleting cloudfront key value");

    diesel::delete(files.filter(file_site_id.eq(site.id.clone())))
        .execute(&mut conn)
        .expect("Error deleting files");

    diesel::delete(sites.filter(site_id.eq(site.id.clone())))
        .execute(&mut conn)
        .expect("Error deleting site");

    HttpResponse::Ok().json(json!({
        "message": format!("Site deleted successfully")
    }))
}

pub async fn list_sites(pool: web::Data<DbPool>) -> impl Responder {
    use crate::schema::sites::dsl::*;

    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let sites_list: Vec<Site> = sites
        .select(Site::as_select())
        .load::<Site>(&mut conn)
        .expect("Error loading sites");

    HttpResponse::Ok().json(serde_json::json!({
        "sites": sites_list,
        "total": sites_list.len(),
    }))
}

pub async fn get_site(path_data: web::Path<String>, pool: web::Data<DbPool>) -> impl Responder {
    use crate::schema::files::dsl::{files, site_id as file_site_id};
    use crate::schema::sites::dsl::*;

    let site_id = path_data.into_inner();
    println!("Site id: {}", site_id);
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let site: Site = match sites
        .filter(id.eq(site_id.clone()))
        .select(Site::as_select())
        .first(&mut conn)
    {
        Ok(site) => site,
        Err(err) => {
            println!("{:?}", err);
            return HttpResponse::NotFound().finish();
        }
    };

    let files_list: Vec<File> = files
        .filter(file_site_id.eq(site_id.clone()))
        .select(File::as_select())
        .load::<File>(&mut conn)
        .expect("Error loading files");

    HttpResponse::Ok().json(serde_json::json!({
        "site": site,
        "files": files_list,
        "total_files": files_list.len(),
    }))
}

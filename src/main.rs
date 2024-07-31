mod config;
mod db;
mod handlers;
mod models;
mod schema;
mod services;
mod utils;

use crate::db::establish_connection_pool;
use actix_web::{web, App, HttpServer, Responder};
use aws_config::{BehaviorVersion, Region};
use handlers::sites;
use services::{cloudfront_key_value, dynamodb, s3};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::Config::new();
    let pool = establish_connection_pool();

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(config.aws_region.clone()))
        .load()
        .await;

    let s3_client = s3::Client::new(&aws_config, &config.aws_s3_bucket_name);
    let cloudfront_kvs_client =
        cloudfront_key_value::Client::new(&aws_config, &config.aws_cloudfront_kvs_arn);
    let dynamodb_client = dynamodb::Client::new(&aws_config, &config.aws_dynamodb_table_name);

    let port = 8080;
    let address = format!("127.0.0.1:{}", port);
    println!("Server started at http://{}", address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .app_data(web::Data::new(cloudfront_kvs_client.clone()))
            .app_data(web::Data::new(dynamodb_client.clone()))
            .route("/sites", web::get().to(sites::list_sites))
            .route("/sites", web::post().to(sites::create_site))
            .route("/sites/{site_id}", web::get().to(sites::get_site))
            .route("/sites/{site_id}", web::put().to(sites::update_site))
            .route("/sites/{site_id}", web::delete().to(sites::delete_site))
    })
    .bind(address)?
    .workers(2)
    .run()
    .await
}

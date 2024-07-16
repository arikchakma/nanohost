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

#[actix_web::get("/")]
async fn greet(req: actix_web::HttpRequest) -> impl Responder {
    // log the request
    println!("{:?}", req);
    format!("Hello, world!")
}

// #[derive(Serialize)]
// struct CreateTodoResponse {
//     id: String,
// }

// #[derive(Deserialize)]
// struct CreateTodoBody {
//     title: String,
//     completed: bool,
// }

// async fn create_todo(
//     pool: web::Data<DbPool>,
//     todo_data: web::Json<CreateTodoBody>,
// ) -> impl Responder {
//     use crate::schema::todos::dsl::*;

//     let todo = todo_data.into_inner();
//     let mut conn = pool.get().expect("couldn't get db connection from pool");

//     let now = Utc::now().naive_utc();
//     let new_todo = Todo {
//         id: Ulid::new().to_string(),
//         title: todo.title.clone(),
//         completed: todo.completed.clone(),
//         completed_at: None,
//         created_at: now,
//         updated_at: now,
//     };

//     diesel::insert_into(todos)
//         .values(&new_todo)
//         .execute(&mut conn)
//         .expect("Error saving new todo");

//     HttpResponse::Created().json(CreateTodoResponse {
//         id: new_todo.id.clone(),
//     })
// }

// async fn get_todo(path: web::Path<String>, pool: web::Data<DbPool>) -> impl Responder {
//     use crate::schema::todos::dsl::*;

//     let todo_id = path.into_inner();
//     let mut conn = pool.get().expect("couldn't get db connection from pool");

//     let todo = match todos.filter(id.eq(todo_id)).first::<Todo>(&mut conn) {
//         Ok(todo) => todo,
//         Err(_) => {
//             return HttpResponse::NotFound().finish();
//         }
//     };

//     HttpResponse::Ok().json(todo)
// }

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
    let dynamodb_client = dynamodb::Client::new(&aws_config, "sites");

    let port = 8080;
    let address = format!("127.0.0.1:{}", port);
    println!("Server started at http://{}", address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .app_data(web::Data::new(cloudfront_kvs_client.clone()))
            .app_data(web::Data::new(dynamodb_client.clone()))
            .service(greet)
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

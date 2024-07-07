mod db;
mod models;
mod schema;

use crate::db::establish_connection_pool;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use db::DbPool;
use diesel::prelude::*;
use models::Todo;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[actix_web::get("/")]
async fn greet() -> impl Responder {
    format!("Hello, world!")
}

#[derive(Serialize)]
struct CreateTodoResponse {
    id: String,
}

#[derive(Deserialize)]
struct CreateTodoBody {
    title: String,
    completed: bool,
}

async fn create_todo(
    pool: web::Data<DbPool>,
    todo_data: web::Json<CreateTodoBody>,
) -> impl Responder {
    use crate::schema::todos::dsl::*;

    let todo = todo_data.into_inner();
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let now = Utc::now().naive_utc();
    let new_todo = Todo {
        id: Ulid::new().to_string(),
        title: todo.title.clone(),
        completed: todo.completed.clone(),
        completed_at: None,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(todos)
        .values(&new_todo)
        .execute(&mut conn)
        .expect("Error saving new todo");

    HttpResponse::Created().json(CreateTodoResponse {
        id: new_todo.id.clone(),
    })
}

async fn get_todo(path: web::Path<String>, pool: web::Data<DbPool>) -> impl Responder {
    use crate::schema::todos::dsl::*;

    let todo_id = path.into_inner();
    let mut conn = pool.get().expect("couldn't get db connection from pool");

    let todo = todos
        .filter(id.eq(todo_id))
        .first::<Todo>(&mut conn)
        .expect("Error loading todo");

    HttpResponse::Ok().json(todo)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = establish_connection_pool();

    let port = 8080;
    let address = format!("127.0.0.1:{}", port);
    println!("Server started at http://{}", address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(greet)
            .route("/todos", web::post().to(create_todo))
            .route("/todos/{id}", web::get().to(get_todo))
    })
    .bind(address)?
    .workers(2)
    .run()
    .await
}

use actix_web::{App, HttpServer, Responder};

#[actix_web::get("/")]
async fn greet() -> impl Responder {
    format!("Hello, world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = 8080;
    println!("Server started at http://localhost:{}", port);

    HttpServer::new(|| App::new().service(greet))
        .bind(("127.0.0.1", port))?
        .workers(2)
        .run()
        .await
}

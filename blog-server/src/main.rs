use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use tracing::{error, info};
use crate::domain::error::AppError;
mod domain;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let rest = tokio::spawn({
        async move { run_server().await }
    });

    tokio::select! {
        res = rest => {
            error!("Rest server stopped unexpectedly: {:?}", res);
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl-C received, shutting down");
        }
    }

    info!("Blog server shut down");

    Ok(())
}
#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-web with Tokio!")
}


async fn run_server() -> Result<(), AppError> {
    HttpServer::new(|| {
        App::new()
            .service(hello) // Register the handler
    })
        .bind(("127.0.0.1", 8080)).unwrap_or_else(|err| panic!("IO Error: {}", err))
        .run()
        .await.unwrap_or_else(|err| panic!("IO Error: {}", err));

    Ok(())
}
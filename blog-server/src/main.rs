use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use tracing::{error, info};
use crate::domain::error::AppError;
use crate::handlers::hello;
use crate::infrasturcture::config::AppConfig;
use crate::infrasturcture::database::{create_pool, run_migrations};

mod domain;
mod handlers;
mod infrasturcture;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = AppConfig::from_env().expect("invalid configuration");

    let pool = create_pool(&config.database_url)
        .await
        .expect("failed to connect to database");
    run_migrations(&pool)
        .await
        .expect("failed to run migrations");

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
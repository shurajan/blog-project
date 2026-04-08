use actix_web::{App, HttpServer};
use tracing::{error, info};
use crate::domain::error::AppError;
use crate::handlers::hello;
use crate::infrasturcture::config::AppConfig;
use crate::infrasturcture::database::{create_pool, run_migrations};
use crate::infrasturcture::logging::init_logging;

mod domain;
mod handlers;
mod infrasturcture;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging()?;
    info!("starting blog server");
    let config = AppConfig::from_env()?;

    let pool = create_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let rest = tokio::spawn({
        async move { run_server().await }
    });

    tokio::select! {
        res = rest => {
            error!("rest server stopped unexpectedly: {:?}", res);
        },
        _ = tokio::signal::ctrl_c() => {
            info!("ctrl-c received, shutting down");
        }
    }

    info!("blog server shut down");

    Ok(())
}


async fn run_server() -> Result<(), AppError> {
    HttpServer::new(|| {
        App::new()
            .service(hello) // Register the handler
    })
        .bind(("127.0.0.1", 8080)).unwrap_or_else(|err| panic!("IO Error: {}", err))
        .run()
        .await?;

    Ok(())
}
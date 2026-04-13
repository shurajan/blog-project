use crate::application::auth_service::AuthService;
use crate::data::user_repository::UserRepository;
use crate::domain::error::AppError;
use crate::domain::user::NewUser;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::database::{create_pool, run_migrations};
use crate::infrastructure::jwt::JwtService;
use crate::infrastructure::logging::init_logging;
use crate::presentation::auth_http_handlers::{login, register};
use actix_web::{App, HttpServer, web};
use std::sync::Arc;
use tracing::{error, info, warn};

mod application;
mod data;
mod domain;
mod infrastructure;
mod presentation;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging()?;
    info!("starting blog server");
    let config = AppConfig::from_env()?;

    let pool = create_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let jwt_service = Arc::new(JwtService::new(&config.jwt_secret));

    let user_repository = UserRepository::new(pool.clone());
    let auth_service = Arc::new(AuthService::new(user_repository.clone(), jwt_service));


    let rest = tokio::spawn({
        let auth_service = auth_service.clone();
        async move { run_server(auth_service).await }
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



async fn run_server(auth_service: Arc<AuthService>) -> Result<(), AppError> {
    let auth_data = web::Data::new(auth_service);

    HttpServer::new(move || App::new().app_data(auth_data.clone())
        .service(register)
        .service(login))
        .bind(("127.0.0.1", 8080))
        .unwrap_or_else(|err| panic!("IO Error: {}", err))
        .run()
        .await?;

    Ok(())
}

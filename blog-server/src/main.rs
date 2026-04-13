use std::sync::Arc;
use actix_web::{web, App, HttpServer};
use tracing::{error, info, warn};
use crate::application::auth_service::AuthService;
use crate::data::user_repository::UserRepository;
use crate::domain::error::AppError;
use crate::domain::user::NewUser;
use crate::presentation::auth_http_handlers::{ register};
use crate::infrasturcture::config::AppConfig;
use crate::infrasturcture::database::{create_pool, run_migrations};
use crate::infrasturcture::jwt::JwtService;
use crate::infrasturcture::logging::init_logging;

mod domain;
mod infrasturcture;
mod data;
mod application;
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
    let auth_service = AuthService::new(user_repository.clone(), jwt_service);

    // TODO: убрать перед сдачей
    if let Err(e) = seed_test_users(&user_repository).await {
        warn!("failed to seed test users: {:?}", e);
    }

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

/// Вставляет несколько тестовых пользователей.
async fn seed_test_users(repo: &UserRepository) -> Result<(), AppError> {
    // Заглушка вместо реального хеша — потом замените на bcrypt/argon2.
    let fake_hash = "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder";

    let users = [
        ("alice", "alice@example.com"),
        ("bob",   "bob@example.com"),
        ("carol", "carol@example.com"),
    ];

    for (username, email) in users {
        let new_user = NewUser {
            username: username.to_string(),
            email: email.to_string(),
            password_hash: fake_hash.to_string(),
        };

        match repo.create(new_user).await {
            Ok(user) => info!("created user: {:?}", user),
            Err(e)   => warn!("failed to seed {}: {:?}", email, e),
        }
    }

    Ok(())
}

async fn run_server(auth_service: AuthService) -> Result<(), AppError> {

    let auth_data = web::Data::new(auth_service);

    HttpServer::new(move || {
        App::new()
            .app_data(auth_data.clone())
            .service(register)
    })
        .bind(("127.0.0.1", 8080))
        .unwrap_or_else(|err| panic!("IO Error: {}", err))
        .run()
        .await?;

    Ok(())
}
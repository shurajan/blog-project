use actix_web::{App, HttpServer};
use sqlx::PgPool;
use tracing::{error, info, warn};
use crate::domain::error::AppError;
use crate::handlers::hello;
use crate::infrasturcture::config::AppConfig;
use crate::infrasturcture::database::{create_pool, run_migrations};
use crate::infrasturcture::logging::init_logging;

mod domain;
mod handlers;
mod infrasturcture;
mod data;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging()?;
    info!("starting blog server");
    let config = AppConfig::from_env()?;

    let pool = create_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    // TODO: убрать перед перед сдачей
    if let Err(e) = seed_test_users(&pool).await {
        warn!("failed to seed test users: {:?}", e);
    }

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

/// Вставляет несколько тестовых пользователей.
async fn seed_test_users(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Заглушка вместо реального хеша — потом замените на bcrypt/argon2.
    let fake_hash = "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder";

    let users = [
        ("alice", "alice@example.com"),
        ("bob",   "bob@example.com"),
        ("carol", "carol@example.com"),
    ];

    for (username, email) in users {
        let result = sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            ON CONFLICT (email) DO NOTHING
            "#,
        )
            .bind(username)
            .bind(email)
            .bind(fake_hash)
            .execute(pool)
            .await?;

        if result.rows_affected() > 0 {
            info!("seeded user {} <{}>", username, email);
        } else {
            info!("user {} <{}> already exists, skipped", username, email);
        }
    }

    Ok(())
}

async fn run_server() -> Result<(), AppError> {
    HttpServer::new(|| {
        App::new()
            .service(hello)
    })
        .bind(("127.0.0.1", 8080)).unwrap_or_else(|err| panic!("IO Error: {}", err))
        .run()
        .await?;

    Ok(())
}
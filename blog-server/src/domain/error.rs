use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("user \"{username}\" not found")]
    UserNotFound { username: String },

    #[error("invalid datetime on server")]
    InvalidDatetime,
}

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            "internal server error".to_string()
        } else {
            self.to_string()
        };

        HttpResponse::build(status).json(json!({
            "error": message,
            "status": status.as_u16(),
        }))
    }
}

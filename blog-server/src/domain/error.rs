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

    #[error("invalid datetime on server")]
    InvalidDatetime,

    #[error("user \"{username}\" not found")]
    UserNotFound { username: String },

    #[error("user with the same username or/and email already registered")]
    UserAlreadyExists,

    #[error("invalid username or password")]
    InvalidCredentials,

    #[error("post not found: id={id}")]
    PostNotFound { id: i64 },

    #[error("access forbidden")]
    Forbidden,

    #[error("unauthorized")]
    Unauthorized,

    #[error("validation error: {0}")]
    Validation(String),
}

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;
use tonic::Status;

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::UserAlreadyExists => StatusCode::CONFLICT,
            AppError::UserNotFound { .. } => StatusCode::UNAUTHORIZED,
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Jwt(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::PostNotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
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

impl From<AppError> for Status {
    fn from(e: AppError) -> Self {
        match e {
            AppError::UserNotFound { .. }
            | AppError::PostNotFound { .. }      => Status::not_found(e.to_string()),

            AppError::InvalidCredentials
            | AppError::Unauthorized
            | AppError::Jwt(_)                   => Status::unauthenticated(e.to_string()),

            AppError::Forbidden                  => Status::permission_denied(e.to_string()),

            AppError::UserAlreadyExists          => Status::already_exists(e.to_string()),

            AppError::Validation(_)              => Status::invalid_argument(e.to_string()),

            _                                    => Status::internal("internal server error".to_string()),
        }
    }
}
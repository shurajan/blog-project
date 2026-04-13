use std::sync::Arc;
use crate::application::auth_service::AuthService;
use crate::domain::error::AppError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, Responder, get, post, web};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[post("/auth/register")]
async fn register(
    service: web::Data<Arc<AuthService>>,
    payload: web::Json<RegisterRequest>,
) -> Result<impl Responder, AppError> {
    let RegisterRequest {
        username,
        email,
        password,
    } = payload.into_inner();
    let user_and_token = service.register(username, email, password).await?;

    info!(user_id = %user_and_token.user.id, username = %user_and_token.user.username, "user registered");

    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(user_and_token))
}

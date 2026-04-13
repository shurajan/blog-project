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

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[post("/api/auth/register")]
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

    info!(user_id = %user_and_token.user.id,  "user registered");

    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(user_and_token))
}

#[post("/api/auth/login")]
async fn login(
    service: web::Data<Arc<AuthService>>,
    payload: web::Json<LoginRequest>,
) -> Result<impl Responder, AppError> {
    let LoginRequest { username, password } = payload.into_inner();
    let user_and_token = service.login(&username, &password).await?;

    info!(user_id = %user_and_token.user.id, "user logged in");
    Ok(HttpResponse::Ok().json(user_and_token))
}
use crate::application::auth_service::AuthService;
use crate::domain::error::AppError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, Responder, post, web};
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[post("/register")]
pub(crate) async fn register(
    service: web::Data<Arc<AuthService>>,
    payload: web::Json<RegisterRequest>,
) -> Result<impl Responder, AppError> {
    let RegisterRequest {
        username,
        email,
        password,
    } = payload.into_inner();
    let user_and_token = service.register(username, email, password).await?;

    debug!(user_id = %user_and_token.user.id,  "user registered");

    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(user_and_token))
}

#[post("/login")]
pub(crate) async fn login(
    service: web::Data<Arc<AuthService>>,
    payload: web::Json<LoginRequest>,
) -> Result<impl Responder, AppError> {
    let LoginRequest { username, password } = payload.into_inner();
    let user_and_token = service.login(username, password).await?;

    debug!(user_id = %user_and_token.user.id, "user logged in");
    Ok(HttpResponse::Ok().json(user_and_token))
}

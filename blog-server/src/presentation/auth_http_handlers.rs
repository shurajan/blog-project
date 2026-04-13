use actix_web::{get, post, web, HttpResponse, HttpResponseBuilder, Responder};
use actix_web::http::StatusCode;
use serde::Deserialize;
use tracing::info;
use crate::application::auth_service::AuthService;
use crate::domain::error::AppError;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[post("/auth/register")]
async fn register(
    service: web::Data<AuthService>,
    payload: web::Json<RegisterRequest>,
) -> Result<impl Responder, AppError> {

    let RegisterRequest { username, email, password } = payload.into_inner();
    let user_and_token = service.register(username, email, password).await?;

    info!(user_id = %user_and_token.user.id, email = %user_and_token.user.email, "user registered");
    
    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(user_and_token))
}
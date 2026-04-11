use actix_web::{get, post, web, HttpResponse, HttpResponseBuilder, Responder};
use actix_web::http::StatusCode;
use tracing::info;
use crate::application::auth_service::AuthService;
use crate::domain::error::AppError;
use crate::dto::RegisterRequest;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-web with Tokio!")
}


#[post("/auth/register")]
async fn register(
    service: web::Data<AuthService>,
    payload: web::Json<RegisterRequest>,
) -> Result<impl Responder, AppError> {

    let RegisterRequest { username, email, password } = payload.into_inner();
    let user = service.register(username, email, password).await?;

    info!(user_id = %user.id, email = %user.email, "user registered");
    
    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(user))
}
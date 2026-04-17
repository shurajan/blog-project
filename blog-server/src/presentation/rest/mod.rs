use actix_web::{get,HttpResponse, Responder};
use tracing::debug;

pub mod auth_http_handlers;
pub mod middleware;
pub mod posts_http_handlers;


#[get("")]
async fn health() -> impl Responder {
    debug!("health check");
    HttpResponse::Ok().finish()
}
use actix_web::{HttpResponse, Responder, get};
use tracing::debug;

pub(crate) mod auth_http_handlers;
pub(crate) mod middleware;
pub(crate) mod posts_http_handlers;

#[get("")]
pub(crate) async fn health() -> impl Responder {
    debug!("health check");
    HttpResponse::Ok().finish()
}

use crate::application::post_service::PostService;
use crate::domain::error::AppError;
use crate::presentation::rest::middleware::AuthUser;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, Responder, delete, get, post, put, web};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct CreatePostRequest {
    title: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct UpdatePostRequest {
    title: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListPostsQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[post("")]
pub(crate) async fn create_post(
    service: web::Data<Arc<PostService>>,
    user: AuthUser,
    payload: web::Json<CreatePostRequest>,
) -> Result<impl Responder, AppError> {
    let CreatePostRequest { title, content } = payload.into_inner();
    let post = service.create(user.id, title, content).await?;

    debug!(post_id = %post.id, author_id = %post.author_id, "post created");

    Ok(HttpResponseBuilder::new(StatusCode::CREATED).json(post))
}

#[get("/{id}")]
pub(crate) async fn get_post(
    service: web::Data<Arc<PostService>>,
    path: web::Path<i64>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let post = service.get(id).await?;

    Ok(HttpResponse::Ok().json(post))
}

#[put("/{id}")]
pub(crate) async fn update_post(
    service: web::Data<Arc<PostService>>,
    user: AuthUser,
    path: web::Path<i64>,
    payload: web::Json<UpdatePostRequest>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let UpdatePostRequest { title, content } = payload.into_inner();
    let post = service.update(id, user.id, title, content).await?;

    debug!(post_id = %post.id, user_id = %user.id, "post updated");

    Ok(HttpResponse::Ok().json(post))
}

#[delete("/{id}")]
pub(crate) async fn delete_post(
    service: web::Data<Arc<PostService>>,
    user: AuthUser,
    path: web::Path<i64>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    service.delete(id, user.id).await?;

    debug!(post_id = %id, user_id = %user.id, "post deleted");

    Ok(HttpResponse::NoContent().finish())
}

#[get("")]
pub(crate) async fn list_posts(
    service: web::Data<Arc<PostService>>,
    query: web::Query<ListPostsQuery>,
) -> Result<impl Responder, AppError> {
    let ListPostsQuery { limit, offset } = query.into_inner();
    let page = service.list(limit, offset).await?;

    Ok(HttpResponse::Ok().json(json!({
        "posts":  page.posts,
        "total":  page.total,
        "limit":  page.limit,
        "offset": page.offset,
    })))
}

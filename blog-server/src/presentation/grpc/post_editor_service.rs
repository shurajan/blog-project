use crate::presentation::grpc::middleware::AuthUser;
use blog_grpc::post_editor_service_server::PostEditorService;
use blog_grpc::{CreatePostRequest, DeletePostRequest, Post, UpdatePostRequest};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::debug;

pub(crate) struct PostEditorApi {
    service: Arc<crate::application::post_service::PostService>,
}

impl PostEditorApi {
    pub(crate) fn new(service: Arc<crate::application::post_service::PostService>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl PostEditorService for PostEditorApi {
    async fn create_post(
        &self,
        request: Request<CreatePostRequest>,
    ) -> Result<Response<Post>, Status> {
        let user = auth_user(&request)?;
        let CreatePostRequest { title, content } = request.into_inner();

        let post = self.service.create(user.id, title, content).await?;
        debug!(post_id = %post.id, author_id = %post.author_id, "post created");

        Ok(Response::new(post.into()))
    }

    async fn update_post(
        &self,
        request: Request<UpdatePostRequest>,
    ) -> Result<Response<Post>, Status> {
        let user = auth_user(&request)?;
        let UpdatePostRequest { id, title, content } = request.into_inner();

        let post = self.service.update(id, user.id, title, content).await?;
        debug!(post_id = %post.id, user_id = %user.id, "post updated");

        Ok(Response::new(post.into()))
    }

    async fn delete_post(
        &self,
        request: Request<DeletePostRequest>,
    ) -> Result<Response<()>, Status> {
        let user = auth_user(&request)?;
        let DeletePostRequest { id } = request.into_inner();

        self.service.delete(id, user.id).await?;
        debug!(post_id = %id, user_id = %user.id, "post deleted");

        Ok(Response::new(()))
    }
}

fn auth_user<T>(request: &Request<T>) -> Result<AuthUser, Status> {
    request
        .extensions()
        .get::<AuthUser>()
        .cloned()
        .ok_or_else(|| Status::unauthenticated("missing auth"))
}

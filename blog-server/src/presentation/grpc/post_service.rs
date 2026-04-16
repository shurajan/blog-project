use blog_grpc::post_service_server::PostService;
use blog_grpc::{GetPostRequest, ListPostsRequest, ListPostsResponse, Post};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct PostApi {
    service: Arc<crate::application::post_service::PostService>,
}

impl PostApi {
    pub fn new(service: Arc<crate::application::post_service::PostService>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl PostService for PostApi {
    async fn get_post(&self, request: Request<GetPostRequest>) -> Result<Response<Post>, Status> {
        let GetPostRequest { id } = request.into_inner();
        let post = self.service.get(id).await?;
        Ok(Response::new(post.into()))
    }

    async fn list_posts(
        &self,
        request: Request<ListPostsRequest>,
    ) -> Result<Response<ListPostsResponse>, Status> {
        let ListPostsRequest { limit, offset } = request.into_inner();
        let page = self.service.list(limit, offset).await?;

        Ok(Response::new(ListPostsResponse {
            posts: page.posts.into_iter().map(Into::into).collect(),
            total: page.total,
            limit: page.limit,
            offset: page.offset,
        }))
    }
}

use crate::{client::BlogClient, error::ClientError, model::*};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tonic::{Request, metadata::MetadataValue, transport::Channel};

pub struct GrpcClient {
    post_public: blog_grpc::post_service_client::PostServiceClient<Channel>,
    post_editor: blog_grpc::post_editor_service_client::PostEditorServiceClient<Channel>,
    auth: blog_grpc::auth_service_client::AuthServiceClient<Channel>,
}

impl GrpcClient {
    /// Creates a gRPC client by opening a channel to the provided endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, ClientError> {
        let ch = Channel::from_shared(endpoint.to_string())
            .map_err(|e| ClientError::Transport(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(Self {
            post_public: blog_grpc::post_service_client::PostServiceClient::new(ch.clone()),
            post_editor: blog_grpc::post_editor_service_client::PostEditorServiceClient::new(
                ch.clone(),
            ),
            auth: blog_grpc::auth_service_client::AuthServiceClient::new(ch),
        })
    }
}

fn with_bearer<T>(mut req: Request<T>, token: &AuthToken) -> Result<Request<T>, ClientError> {
    let val: MetadataValue<_> = format!("Bearer {}", token.0)
        .parse()
        .map_err(|_| ClientError::InvalidArgument("bad token".into()))?;
    req.metadata_mut().insert("authorization", val);
    Ok(req)
}

fn ts_to_dt(ts: Option<prost_types::Timestamp>) -> DateTime<Utc> {
    ts.and_then(|t| DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .unwrap_or_default()
}

impl From<blog_grpc::Post> for Post {
    fn from(p: blog_grpc::Post) -> Self {
        Post {
            id: p.id,
            author_id: p.author_id,
            title: p.title,
            content: p.content,
            created_at: ts_to_dt(p.created_at),
            updated_at: ts_to_dt(p.updated_at),
        }
    }
}

// ── Trait impl ─────────────────────────────────────────────────────
#[async_trait]
impl BlogClient for GrpcClient {
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthToken, i64), ClientError> {
        let mut client = self.auth.clone();
        let resp = client
            .register(blog_grpc::RegisterRequest {
                username: username.to_string(),
                email: email.to_string(),
                password: password.to_string(),
            })
            .await?;
        let inner = resp.into_inner();
        let user_id = inner.user.map(|u| u.id).unwrap_or(0);
        Ok((AuthToken(inner.token), user_id))
    }

    async fn login(&self, username: &str, password: &str) -> Result<AuthToken, ClientError> {
        let mut client = self.auth.clone();
        let resp = client
            .login(blog_grpc::LoginRequest {
                username: username.to_string(),
                password: password.to_string(),
            })
            .await?;
        Ok(AuthToken(resp.into_inner().token))
    }

    async fn get_post(&self, id: i64) -> Result<Post, ClientError> {
        let mut client = self.post_public.clone();
        let resp = client.get_post(blog_grpc::GetPostRequest { id }).await?;
        Ok(resp.into_inner().into())
    }

    async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostPage, ClientError> {
        let mut client = self.post_public.clone();
        let resp = client
            .list_posts(blog_grpc::ListPostsRequest { limit, offset })
            .await?;
        let inner = resp.into_inner();
        Ok(PostPage {
            posts: inner.posts.into_iter().map(Into::into).collect(),
            total: inner.total,
            limit: inner.limit,
            offset: inner.offset,
        })
    }

    async fn create_post(
        &self,
        token: &AuthToken,
        title: &str,
        content: &str,
    ) -> Result<Post, ClientError> {
        let mut client = self.post_editor.clone();
        let req = with_bearer(
            Request::new(blog_grpc::CreatePostRequest {
                title: title.to_string(),
                content: content.to_string(),
            }),
            token,
        )?;
        let resp = client.create_post(req).await?;
        Ok(resp.into_inner().into())
    }

    async fn update_post(
        &self,
        token: &AuthToken,
        id: i64,
        patch: PostPatch,
    ) -> Result<Post, ClientError> {
        let mut client = self.post_editor.clone();
        let req = with_bearer(
            Request::new(blog_grpc::UpdatePostRequest {
                id,
                title: patch.title,
                content: patch.content,
            }),
            token,
        )?;
        let resp = client.update_post(req).await?;
        Ok(resp.into_inner().into())
    }

    async fn delete_post(&self, token: &AuthToken, id: i64) -> Result<(), ClientError> {
        let mut client = self.post_editor.clone();
        let req = with_bearer(Request::new(blog_grpc::DeletePostRequest { id }), token)?;
        client.delete_post(req).await?;
        Ok(())
    }
}

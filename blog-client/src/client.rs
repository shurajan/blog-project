use crate::{error::ClientError, model::*};
use async_trait::async_trait;

#[async_trait]
pub trait BlogClient: Send + Sync {
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthToken, i64), ClientError>;
    async fn login(&self, username: &str, password: &str) -> Result<AuthToken, ClientError>;

    async fn get_post(&self, id: i64) -> Result<Post, ClientError>;
    async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostPage, ClientError>;

    async fn create_post(
        &self,
        token: &AuthToken,
        title: &str,
        content: &str,
    ) -> Result<Post, ClientError>;
    async fn update_post(
        &self,
        token: &AuthToken,
        id: i64,
        patch: PostPatch,
    ) -> Result<Post, ClientError>;
    async fn delete_post(&self, token: &AuthToken, id: i64) -> Result<(), ClientError>;
}

use crate::{error::ClientError, model::*};
use async_trait::async_trait;

#[async_trait]
pub trait BlogClient: Send + Sync {
    /// Registers a new user and returns the issued auth token together with the created user id.
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthToken, i64), ClientError>;

    /// Authenticates an existing user and returns a fresh auth token.
    async fn login(&self, username: &str, password: &str) -> Result<AuthToken, ClientError>;

    /// Fetches a single post by its identifier.
    async fn get_post(&self, id: i64) -> Result<Post, ClientError>;

    /// Returns a paginated list of posts using optional limit and offset values.
    async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostPage, ClientError>;

    /// Creates a new post on behalf of the authenticated user.
    async fn create_post(
        &self,
        token: &AuthToken,
        title: &str,
        content: &str,
    ) -> Result<Post, ClientError>;

    /// Updates the selected post with the provided partial patch.
    async fn update_post(
        &self,
        token: &AuthToken,
        id: i64,
        patch: PostPatch,
    ) -> Result<Post, ClientError>;

    /// Deletes the selected post on behalf of the authenticated user.
    async fn delete_post(&self, token: &AuthToken, id: i64) -> Result<(), ClientError>;
}

use crate::{client::BlogClient, error::ClientError, model::*};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tonic::{Request, metadata::MetadataValue, transport::Channel};
use url::Url;

pub struct HttpClient {
    base_url: Url,
}

impl HttpClient {
    pub async fn connect(url: &str) -> Result<Self, ClientError> {
        let base_url = Url::parse(url)?;
        todo!()
    }
}

#[async_trait]
impl BlogClient for HttpClient {
    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthToken, i64), ClientError> {
        todo!()
    }

    async fn login(&self, username: &str, password: &str) -> Result<AuthToken, ClientError> {
        todo!()
    }

    async fn get_post(&self, id: i64) -> Result<Post, ClientError> {
        todo!()
    }

    async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostPage, ClientError> {
        todo!()
    }

    async fn create_post(
        &self,
        token: &AuthToken,
        title: &str,
        content: &str,
    ) -> Result<Post, ClientError> {
        todo!()
    }

    async fn update_post(
        &self,
        token: &AuthToken,
        id: i64,
        patch: PostPatch,
    ) -> Result<Post, ClientError> {
        todo!()
    }

    async fn delete_post(&self, token: &AuthToken, id: i64) -> Result<(), ClientError> {
        todo!()
    }
}

use crate::{
    client::BlogClient,
    error::{ClientError, map_status},
    model::*,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use url::Url;

// ── Private DTOs ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct AuthResponseDto {
    token: String,
    user: Option<UserDto>,
}

#[derive(Deserialize)]
struct UserDto {
    id: i64,
}

#[derive(Deserialize)]
struct PostDto {
    id: i64,
    author_id: i64,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct PostPageDto {
    posts: Vec<PostDto>,
    total: i64,
    limit: i64,
    offset: i64,
}

// ── DTO → Model ────────────────────────────────────────────────────

impl From<PostDto> for Post {
    fn from(dto: PostDto) -> Self {
        Self {
            id: dto.id,
            author_id: dto.author_id,
            title: dto.title,
            content: dto.content,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }
}

// ── Client ─────────────────────────────────────────────────────────

pub struct HttpClient {
    base_url: Url,
    http: Client,
}

impl HttpClient {
    pub async fn connect(url: &str) -> Result<Self, ClientError> {
        let base_url = Url::parse(url).map_err(|e| ClientError::Transport(e.to_string()))?;
        let http = Client::new();

        let resp = http
            .get(base_url.join("/health").map_err(|e| ClientError::Transport(e.to_string()))?)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ClientError::Transport(format!(
                "health check failed: {}",
                resp.status()
            )));
        }

        Ok(Self { base_url, http })
    }

    fn url(&self, path: &str) -> Result<Url, ClientError> {
        self.base_url
            .join(path)
            .map_err(|e| ClientError::Transport(e.to_string()))
    }
}

// ── Trait impl ─────────────────────────────────────────────────────

#[async_trait]
impl BlogClient for HttpClient {
    // ── Auth ───────────────────────────────────────────────────────

    async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthToken, i64), ClientError> {
        let resp = self
            .http
            .post(self.url("/api/auth/register")?)
            .json(&json!({
                "username": username,
                "email": email,
                "password": password,
            }))
            .send()
            .await?;

        map_status(resp.status())?;

        let dto: AuthResponseDto = resp.json().await?;
        let user = dto
            .user
            .ok_or(ClientError::Other("missing user in response".into()))?;

        Ok((AuthToken(dto.token), user.id))
    }

    async fn login(&self, username: &str, password: &str) -> Result<AuthToken, ClientError> {
        let resp = self
            .http
            .post(self.url("/api/auth/login")?)
            .json(&json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await?;

        map_status(resp.status())?;

        let dto: AuthResponseDto = resp.json().await?;
        Ok(AuthToken(dto.token))
    }

    // ── Posts (public) ─────────────────────────────────────────────

    async fn get_post(&self, id: i64) -> Result<Post, ClientError> {
        let resp = self
            .http
            .get(self.url(&format!("/api/posts/{id}"))?)
            .send()
            .await?;

        map_status(resp.status())?;

        let dto: PostDto = resp.json().await?;
        Ok(dto.into())
    }

    async fn list_posts(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostPage, ClientError> {
        let mut url = self.url("/api/posts")?;

        if let Some(l) = limit {
            url.query_pairs_mut().append_pair("limit", &l.to_string());
        }
        if let Some(o) = offset {
            url.query_pairs_mut().append_pair("offset", &o.to_string());
        }

        let resp = self.http.get(url).send().await?;

        map_status(resp.status())?;

        let dto: PostPageDto = resp.json().await?;
        Ok(PostPage {
            posts: dto.posts.into_iter().map(Into::into).collect(),
            total: dto.total,
            limit: dto.limit,
            offset: dto.offset,
        })
    }

    // ── Posts (requires token) ─────────────────────────────────────

    async fn create_post(
        &self,
        token: &AuthToken,
        title: &str,
        content: &str,
    ) -> Result<Post, ClientError> {
        let resp = self
            .http
            .post(self.url("/api/posts")?)
            .bearer_auth(&token.0)
            .json(&json!({
                "title": title,
                "content": content,
            }))
            .send()
            .await?;

        map_status(resp.status())?;

        let dto: PostDto = resp.json().await?;
        Ok(dto.into())
    }

    async fn update_post(
        &self,
        token: &AuthToken,
        id: i64,
        patch: PostPatch,
    ) -> Result<Post, ClientError> {
        let resp = self
            .http
            .put(self.url(&format!("/api/posts/{id}"))?)
            .bearer_auth(&token.0)
            .json(&json!({
                "title": patch.title,
                "content": patch.content,
            }))
            .send()
            .await?;

        map_status(resp.status())?;

        let dto: PostDto = resp.json().await?;
        Ok(dto.into())
    }

    async fn delete_post(&self, token: &AuthToken, id: i64) -> Result<(), ClientError> {
        let resp = self
            .http
            .delete(self.url(&format!("/api/posts/{id}"))?)
            .bearer_auth(&token.0)
            .send()
            .await?;

        map_status(resp.status())?;
        Ok(())
    }
}

use tracing::info;

use crate::data::post_repository::PostRepository;
use crate::domain::error::AppError;
use crate::domain::post::{NewPost, Post, PostListPage, PostUpdate};

const MAX_PAGE_SIZE: i64 = 100;
const DEFAULT_PAGE_SIZE: i64 = 10;

#[derive(Clone)]
pub struct PostService {
    post_repo: PostRepository,
}

impl PostService {
    pub fn new(post_repo: PostRepository) -> Self {
        Self { post_repo }
    }

    pub async fn create(
        &self,
        author_id: i64,
        title: String,
        content: String,
    ) -> Result<Post, AppError> {
        if title.trim().is_empty() {
            return Err(AppError::Validation("title must not be empty".into()));
        }
        if content.trim().is_empty() {
            return Err(AppError::Validation("content must not be empty".into()));
        }

        let new_post = NewPost { title, content, author_id };
        let post = self.post_repo.create(new_post).await?;

        info!(post_id = %post.id, author_id = %post.author_id, "post created");
        Ok(post)
    }

    pub async fn get(&self, id: i64) -> Result<Post, AppError> {
        self.post_repo.find_by_id(id).await
    }

    pub async fn update(
        &self,
        id: i64,
        user_id: i64,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Post, AppError> {
        if let Some(ref t) = title {
            if t.trim().is_empty() {
                return Err(AppError::Validation("title must not be empty".into()));
            }
        }
        if let Some(ref c) = content {
            if c.trim().is_empty() {
                return Err(AppError::Validation("content must not be empty".into()));
            }
        }
        if title.is_none() && content.is_none() {
            return Err(AppError::Validation("nothing to update".into()));
        }

        let existing = self.post_repo.find_by_id(id).await?;
        if existing.author_id != user_id {
            return Err(AppError::Forbidden);
        }

        let patch = PostUpdate { title, content };
        let post = self.post_repo.update(id, patch).await?;

        info!(post_id = %post.id, user_id = %user_id, "post updated");
        Ok(post)
    }

    pub async fn delete(&self, id: i64, user_id: i64) -> Result<(), AppError> {
        let existing = self.post_repo.find_by_id(id).await?;
        if existing.author_id != user_id {
            return Err(AppError::Forbidden);
        }

        self.post_repo.delete(id).await?;

        info!(post_id = %id, user_id = %user_id, "post deleted");
        Ok(())
    }

    pub async fn list(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PostListPage, AppError> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
        let offset = offset.unwrap_or(0).max(0);

        let (posts, total) =
            tokio::try_join!(self.post_repo.list(limit, offset), self.post_repo.count())?;

        Ok(PostListPage { posts, total, limit, offset })
    }
}

use crate::domain::error::AppError;
use crate::domain::post::{NewPost, Post, PostUpdate};
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostRepository {
    pool: PgPool,
}

impl PostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PostRepository {
    pub async fn create(&self, new: NewPost) -> Result<Post, AppError> {
        sqlx::query_as!(
            Post,
            r#"
            INSERT INTO posts (title, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, author_id, created_at, updated_at
            "#,
            new.title,
            new.content,
            new.author_id,
        )
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Post, AppError> {
        sqlx::query_as!(
            Post,
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
            FROM posts WHERE id = $1
            "#,
            id,
        )
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::PostNotFound { id })
    }

    pub async fn update(&self, id: i64, patch: PostUpdate) -> Result<Post, AppError> {
        sqlx::query_as!(
            Post,
            r#"
            UPDATE posts
            SET title      = COALESCE($2, title),
                content    = COALESCE($3, content),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, title, content, author_id, created_at, updated_at
            "#,
            id,
            patch.title,
            patch.content,
        )
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::PostNotFound { id })
    }

    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"DELETE FROM posts WHERE id = $1"#,
            id,
        )
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::PostNotFound { id });
        }
        Ok(())
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, AppError> {
        sqlx::query_as!(
            Post,
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
            FROM posts
            ORDER BY created_at DESC, id DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset,
        )
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }
    
    pub async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query!(r#"SELECT COUNT(*) as "count!" FROM posts"#)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.count)
    }
}
use crate::domain::error::AppError;
use crate::domain::post::{NewPost, Post, PostUpdate};
use sqlx::PgPool;
use tracing::{debug, instrument, warn};

#[derive(Clone)]
pub(crate) struct PostRepository {
    pool: PgPool,
}

impl PostRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PostRepository {
    #[instrument(skip(self, new), fields(author_id = new.author_id, post_id = tracing::field::Empty), err)]
    pub(crate) async fn create(&self, new: NewPost) -> Result<Post, AppError> {
        let post = sqlx::query_as!(
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
        .map_err(AppError::from)?;

        tracing::Span::current().record("post_id", post.id);
        debug!(
            post_id = post.id,
            author_id = post.author_id,
            "post row inserted"
        );
        Ok(post)
    }

    #[instrument(skip(self), fields(post_id = id), err)]
    pub(crate) async fn find_by_id(&self, id: i64) -> Result<Post, AppError> {
        let post = sqlx::query_as!(
            Post,
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
            FROM posts WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            warn!(post_id = id, "post not found");
            AppError::PostNotFound { id }
        })?;

        debug!(post_id = post.id, "post found");
        Ok(post)
    }

    #[instrument(skip(self, patch), fields(post_id = id), err)]
    pub(crate) async fn update(&self, id: i64, patch: PostUpdate) -> Result<Post, AppError> {
        let post = sqlx::query_as!(
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
        .ok_or_else(|| {
            warn!(post_id = id, "post not found for update");
            AppError::PostNotFound { id }
        })?;

        debug!(post_id = post.id, "post row updated");
        Ok(post)
    }

    #[instrument(skip(self), fields(post_id = id), err)]
    pub(crate) async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query!(r#"DELETE FROM posts WHERE id = $1"#, id,)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            warn!(post_id = id, "post not found for delete");
            return Err(AppError::PostNotFound { id });
        }

        debug!(post_id = id, "post row deleted");
        Ok(())
    }

    #[instrument(skip(self), fields(limit, offset), err)]
    pub(crate) async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, AppError> {
        let posts = sqlx::query_as!(
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
        .map_err(AppError::from)?;

        debug!(count = posts.len(), limit, offset, "posts listed");
        Ok(posts)
    }

    #[instrument(skip(self), err)]
    pub(crate) async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query!(r#"SELECT COUNT(*) as "count!" FROM posts"#)
            .fetch_one(&self.pool)
            .await?;
        debug!(count = row.count, "posts counted");
        Ok(row.count)
    }
}

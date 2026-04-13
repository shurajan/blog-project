use crate::domain::error::AppError;
use crate::domain::user::{NewUser, User};
use sqlx::PgPool;

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl UserRepository {
    pub async fn create(&self, new: NewUser) -> Result<User, AppError> {
        const DUPLICATE_CODE: &str = "23505";

        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, username, email, password_hash, created_at
            "#,
            new.username,
            new.email,
            new.password_hash,
        )
        .fetch_one(&self.pool)
        .await
            .map_err(|err| match err.as_database_error().and_then(|e| e.code()) {
                Some(code) if code == DUPLICATE_CODE => AppError::UserAlreadyExists,
                _ => AppError::from(err),
            })
    }

    pub async fn find_by_username(&self, username: &str) -> Result<User, AppError> {
        sqlx::query_as!(
            User,
            r#"SELECT id, username, email, password_hash, created_at
               FROM users WHERE username = $1"#,
            username,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound {
            username: username.to_string(),
        })
    }
}

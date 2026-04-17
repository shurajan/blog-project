use crate::domain::error::AppError;
use crate::domain::user::{NewUser, User};
use sqlx::PgPool;
use tracing::{debug, instrument, warn};

#[derive(Clone)]
pub(crate) struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl UserRepository {
    #[instrument(skip(self, new), fields(username = %new.username), err)]
    pub(crate) async fn create(&self, new: NewUser) -> Result<User, AppError> {
        const DUPLICATE_CODE: &str = "23505";

        let result = sqlx::query_as!(
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
            Some(code) if code == DUPLICATE_CODE => {
                warn!(username = %new.username, "user already exists");
                AppError::UserAlreadyExists
            }
            _ => AppError::from(err),
        })?;

        debug!(user_id = result.id, username = %result.username, "user row inserted");
        Ok(result)
    }

    #[instrument(skip(self), fields(username = %username), err)]
    pub(crate) async fn find_by_username(&self, username: &str) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"SELECT id, username, email, password_hash, created_at
               FROM users WHERE username = $1"#,
            username,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            warn!(username = %username, "user not found");
            AppError::InvalidCredentials
        })?;

        debug!(user_id = user.id, username = %user.username, "user found");
        Ok(user)
    }
}

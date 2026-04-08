use sqlx::PgPool;
use crate::domain::user::{NewUser, User};

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

    pub async fn create(&self, new: NewUser) -> sqlx::Result<User> {
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
    }

    pub async fn find_by_email(&self, email: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as!(
            User,
            r#"SELECT id, username, email, password_hash, created_at
               FROM users WHERE email = $1"#,
            email,
        )
            .fetch_optional(&self.pool)
            .await
    }
}
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Table `users` — to read from database.
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("email", &self.email)
            .field("password_hash", &"[hidden]")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Clone)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

impl std::fmt::Debug for NewUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("username", &self.username)
            .field("email", &self.email)
            .field("password_hash", &"[hidden]")
            .finish()
    }
}

#[derive(Serialize)]
pub struct UserAndToken {
    pub user: User,
    pub token: String,
}

impl std::fmt::Debug for UserAndToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.user.id)
            .field("username", &self.user.username)
            .field("jwt_token", &"[hidden]")
            .finish()
    }
}
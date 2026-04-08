use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Table `users` — to read from database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

impl NewUser {
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        Self { username, email, password_hash }
    }
}
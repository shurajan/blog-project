use std::sync::Arc;

use tracing::{info};

use crate::data::user_repository::UserRepository;
use crate::domain::{user::User};
use crate::domain::error::AppError;
use crate::domain::user::NewUser;
use crate::infrasturcture::security::hash_password;

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
}

impl AuthService
{
    pub fn new(user_repo: UserRepository) -> Self {
        Self {user_repo}
    }

    pub async fn get_user(&self, username: &str) -> Result<User, AppError> {
        self.user_repo
            .find_by_username(username)
            .await
    }

    pub async fn register(&self, username: String, email: String, password: String) -> Result<User, AppError> {
        let hash = hash_password(&password).map_err(|err| AppError::Internal(err.to_string()))?;
        let new_user = NewUser {
            username: username.to_string(),
            email: email.to_string(),
            password_hash: hash.to_string(),
        };
        let result = self.user_repo.create(new_user).await?;
        info!("created user: {:?}", result);
        Ok(result)
    }
}



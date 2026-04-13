use std::sync::Arc;
use tracing::info;

use crate::data::user_repository::UserRepository;
use crate::domain::error::AppError;
use crate::domain::error::AppError::UserNotFound;
use crate::domain::user::User;
use crate::domain::user::{NewUser, UserAndToken};
use crate::infrastructure::jwt::JwtService;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    jwt_service: Arc<JwtService>,
}

impl AuthService {
    pub fn new(user_repo: UserRepository, jwt_service: Arc<JwtService>) -> Self {
        Self {
            user_repo,
            jwt_service,
        }
    }

    pub async fn get_user(&self, username: &str) -> Result<User, AppError> {
        self.user_repo.find_by_username(username).await
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<UserAndToken, AppError> {
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let new_user = NewUser {
            username,
            email,
            password_hash,
        };
        let user = self.user_repo.create(new_user).await?;

        let token = self
            .jwt_service
            .generate_token(user.id.clone(), user.email.clone())?;

        let result = UserAndToken { user, token };
        info!("created user: {:?}", result);
        Ok(result)
    }
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed).is_ok())
}

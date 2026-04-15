use crate::domain::error::AppError;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        info!("loading config");
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|e| AppError::Config(format!("DATABASE_URL: {e}")))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|e| AppError::Config(format!("JWT_SECRET: {e}")))?;

        Ok(Self {
            database_url,
            jwt_secret,
        })
    }

    pub fn with_database_url(mut self, database_url: String) -> Self {
        self.database_url = database_url;
        self
    }

    pub fn with_jwt_secret(mut self, jwt_secret: String) -> Self {
        self.jwt_secret = jwt_secret;
        self
    }
}

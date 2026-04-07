use serde::Deserialize;
use tracing::info;
use crate::domain::error::AppError;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        info!("loading config");
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|e| AppError::Config(format!("DATABASE_URL: {e}")))?;

        Ok(Self {
            database_url,
        })
    }
}


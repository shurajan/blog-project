use tracing_subscriber::{fmt, EnvFilter};
use crate::domain::error::AppError;

pub fn init_logging() -> Result<(), AppError> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| AppError::Config(format!("logging initiation error: {e}")))?;

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .json()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(())
}


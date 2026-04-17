use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Unable to parse url: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("conflict")]
    Conflict,
    #[error("transport: {0}")]
    Transport(String),
    #[error("unexpected: {0}")]
    Other(String),
}

impl From<tonic::Status> for ClientError {
    fn from(s: tonic::Status) -> Self {
        match s.code() {
            tonic::Code::Unauthenticated => ClientError::Unauthorized,
            tonic::Code::PermissionDenied => ClientError::Forbidden,
            tonic::Code::NotFound => ClientError::NotFound,
            tonic::Code::InvalidArgument => ClientError::InvalidArgument(s.message().to_string()),
            tonic::Code::AlreadyExists => ClientError::Conflict,
            _ => ClientError::Transport(s.to_string()),
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() {
            ClientError::Transport(format!("connection failed: {e}"))
        } else if e.is_timeout() {
            ClientError::Transport(format!("request timed out: {e}"))
        } else if e.is_decode() {
            ClientError::Other(format!("invalid response body: {e}"))
        } else {
            ClientError::Transport(e.to_string())
        }
    }
}

/// Converts an HTTP status code into a client-facing result.
pub fn map_status(status: StatusCode) -> Result<(), ClientError> {
    match status {
        s if s.is_success() => Ok(()),
        StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
        StatusCode::FORBIDDEN => Err(ClientError::Forbidden),
        StatusCode::NOT_FOUND => Err(ClientError::NotFound),
        StatusCode::CONFLICT => Err(ClientError::Conflict),
        StatusCode::BAD_REQUEST => Err(ClientError::InvalidArgument("bad request".into())),
        s => Err(ClientError::Other(format!("unexpected status: {s}"))),
    }
}

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

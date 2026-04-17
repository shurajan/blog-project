pub mod client;
pub mod error;
pub mod model;
mod transport;

pub use client::BlogClient;
pub use error::ClientError;
pub use model::*;
pub use transport::*;

#[derive(Debug, Clone)]
pub enum Transport {
    Http { base_url: String },
    Grpc { endpoint: String },
}

pub async fn connect(cfg: Transport) -> Result<Box<dyn BlogClient>, ClientError> {
    match cfg {
        Transport::Http { base_url } => Ok(Box::new(http::HttpClient::connect(&base_url).await?)),
        Transport::Grpc { endpoint } => Ok(Box::new(grpc::GrpcClient::connect(&endpoint).await?)),
    }
}

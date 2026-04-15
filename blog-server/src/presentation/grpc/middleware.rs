// blog-server/src/presentation/grpc/middleware.rs
use std::sync::Arc;

use tonic::{Request, Status, service::Interceptor};
use tracing::{debug, warn};

use crate::infrastructure::jwt::{Claims, JwtService};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
}

impl From<Claims> for AuthUser {
    fn from(claims: Claims) -> Self {
        Self {
            id: claims.user_id,
            username: claims.username,
        }
    }
}

#[derive(Clone)]
pub struct JwtInterceptor {
    jwt_service: Arc<JwtService>,
}

impl JwtInterceptor {
    pub fn new(jwt_service: Arc<JwtService>) -> Self {
        Self { jwt_service }
    }
}

impl Interceptor for JwtInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let token = match extract_bearer_token(&request) {
            Some(t) => t,
            None => {
                warn!("grpc request rejected: missing or malformed Authorization metadata");
                return Err(Status::unauthenticated(
                    "missing or malformed Authorization metadata",
                ));
            }
        };

        match self.jwt_service.verify_token(&token) {
            Ok(claims) => {
                debug!(user_id = claims.user_id, username = %claims.username, "grpc token verified");
                request.extensions_mut().insert(AuthUser::from(claims));
                Ok(request)
            }
            Err(err) => {
                warn!(error = %err, "grpc request rejected: token verification failed");
                Err(Status::unauthenticated("invalid token"))
            }
        }
    }
}

fn extract_bearer_token<T>(request: &Request<T>) -> Option<String> {
    let header = request.metadata().get("authorization")?;
    let value = header.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

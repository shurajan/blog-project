use crate::presentation::grpc::proto::blog::auth_service_server::AuthService;
use crate::presentation::grpc::proto::blog::{AuthResponse, LoginRequest, RegisterRequest};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::debug;

pub struct AuthApi {
    service: Arc<crate::application::auth_service::AuthService>,
}

impl AuthApi {
    pub fn new(service: Arc<crate::application::auth_service::AuthService>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl AuthService for AuthApi {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let RegisterRequest {
            username,
            email,
            password,
        } = request.into_inner();

        let user_and_token = self.service.register(username, email, password).await?;
        debug!(user_id = %user_and_token.user.id,  "user registered");
        let auth_response = AuthResponse {
            user: Some(user_and_token.user.into()),
            token: user_and_token.token,
        };
        Ok(Response::new(auth_response))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let LoginRequest { username, password } = request.into_inner();

        let user_and_token = self.service.login(username, password).await?;
        debug!(user_id = %user_and_token.user.id,  "user logged in");
        let auth_response = AuthResponse {
            user: None,
            token: user_and_token.token,
        };
        Ok(Response::new(auth_response))
    }
}

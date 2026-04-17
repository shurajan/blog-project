use std::future::{Ready, ready};
use std::rc::Rc;
use std::sync::Arc;

use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest, ResponseError, dev::Payload};
use futures_util::future::LocalBoxFuture;

use tracing::{debug, warn};

use crate::domain::error::AppError;
use crate::infrastructure::jwt::{Claims, JwtService};

#[derive(Debug, Clone)]
pub(crate) struct AuthUser {
    pub(crate) id: i64,
}

impl From<Claims> for AuthUser {
    fn from(claims: Claims) -> Self {
        Self { id: claims.user_id }
    }
}

impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized.into());
        ready(result)
    }
}

pub(crate) struct JwtAuthMiddleware {
    jwt_service: Arc<JwtService>,
}

impl JwtAuthMiddleware {
    pub(crate) fn new(jwt_service: Arc<JwtService>) -> Self {
        Self { jwt_service }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddlewareService {
            service: Rc::new(service),
            jwt_service: self.jwt_service.clone(),
        }))
    }
}

pub(crate) struct JwtAuthMiddlewareService<S> {
    service: Rc<S>,
    jwt_service: Arc<JwtService>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let jwt_service = self.jwt_service.clone();

        Box::pin(async move {
            let path = req.path().to_owned();
            let method = req.method().clone();

            let token = match extract_bearer_token(&req) {
                Some(t) => t,
                None => {
                    warn!(method = %method, path = %path, "request rejected: missing or malformed Authorization header");
                    let (http_req, _) = req.into_parts();
                    let resp = AppError::Unauthorized.error_response();
                    return Ok(ServiceResponse::new(http_req, resp).map_into_right_body());
                }
            };

            match jwt_service.verify_token(&token) {
                Ok(claims) => {
                    debug!(user_id = claims.user_id, username = %claims.username, method = %method, path = %path, "token verified");
                    req.extensions_mut().insert(AuthUser::from(claims));
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(err) => {
                    warn!(method = %method, path = %path, error = %err, "request rejected: token verification failed");
                    let (http_req, _) = req.into_parts();
                    let resp = err.error_response();
                    Ok(ServiceResponse::new(http_req, resp).map_into_right_body())
                }
            }
        })
    }
}

fn extract_bearer_token(req: &ServiceRequest) -> Option<String> {
    let header = req.headers().get(actix_web::http::header::AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

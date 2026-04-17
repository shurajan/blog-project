use crate::application::auth_service::AuthService;
use crate::application::post_service::PostService;
use crate::data::post_repository::PostRepository;
use crate::data::user_repository::UserRepository;
use crate::domain::error::AppError;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::database::{create_pool, run_migrations};
use crate::infrastructure::jwt::JwtService;
use crate::presentation::grpc::auth_service::AuthApi;
use crate::presentation::grpc::middleware::JwtInterceptor;
use crate::presentation::grpc::post_editor_service::PostEditorApi;
use crate::presentation::grpc::post_service::PostApi;
use crate::presentation::rest::auth_http_handlers::{login, register};
use crate::presentation::rest::health;
use crate::presentation::rest::middleware::JwtAuthMiddleware;
use crate::presentation::rest::posts_http_handlers::{
    create_post, delete_post, get_post, list_posts, update_post,
};
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use blog_grpc::auth_service_server::AuthServiceServer;
use blog_grpc::post_editor_service_server::PostEditorServiceServer;
use blog_grpc::post_service_server::PostServiceServer;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    auth_service: Arc<AuthService>,
    post_service: Arc<PostService>,
    jwt_service: Arc<JwtService>,
}

async fn build_app_state(config: &AppConfig) -> Result<AppState, AppError> {
    let pool = create_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let jwt_service = Arc::new(JwtService::new(&config.jwt_secret));

    let user_repository = UserRepository::new(pool.clone());
    let post_repository = PostRepository::new(pool);

    let auth_service = Arc::new(AuthService::new(user_repository, jwt_service.clone()));
    let post_service = Arc::new(PostService::new(post_repository));

    Ok(AppState {
        auth_service,
        post_service,
        jwt_service,
    })
}

/// Starts the REST and gRPC servers and keeps them running until shutdown is requested.
pub async fn run_app(
    config: AppConfig,
    http_port: u16,
    grpc_port: u16,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
    let state = build_app_state(&config).await?;

    run_app_with_state(state, http_port, grpc_port, shutdown).await
}

async fn run_app_with_state(
    state: AppState,
    http_port: u16,
    grpc_port: u16,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
    let mut tasks: JoinSet<(&'static str, Result<(), AppError>)> = JoinSet::new();

    tasks.spawn({
        let auth = state.auth_service.clone();
        let post = state.post_service.clone();
        let jwt = state.jwt_service.clone();
        let shutdown = shutdown.clone();

        async move { ("rest", run_rest(http_port, auth, post, jwt, shutdown).await) }
    });

    tasks.spawn({
        let auth = state.auth_service.clone();
        let post = state.post_service.clone();
        let jwt = state.jwt_service.clone();
        let shutdown = shutdown.clone();

        async move { ("grpc", run_grpc(grpc_port, auth, post, jwt, shutdown).await) }
    });

    tokio::select! {
        _ = shutdown.cancelled() => {
            info!("app: shutdown requested");
        }
        Some(res) = tasks.join_next() => {
            match res {
                Ok((name, Ok(()))) => warn!("{name} server finished unexpectedly"),
                Ok((name, Err(e))) => error!("{name} server failed: {:?}", e),
                Err(e) => error!("task join error: {:?}", e),
            }
        }
    }

    shutdown.cancel();

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok((name, Ok(()))) => info!("{name} server stopped gracefully"),
            Ok((name, Err(e))) => error!("{name} server stopped with error: {:?}", e),
            Err(e) => error!("task join error: {:?}", e),
        }
    }

    Ok(())
}

async fn run_rest(
    port: u16,
    auth_service: Arc<AuthService>,
    post_service: Arc<PostService>,
    jwt_service: Arc<JwtService>,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
    info!(port, "rest: starting server");

    let auth_data = web::Data::new(auth_service);
    let post_data = web::Data::new(post_service);
    let jwt_data = web::Data::new(jwt_service.clone());

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .max_age(3600);

        App::new()
            .app_data(auth_data.clone())
            .app_data(post_data.clone())
            .app_data(jwt_data.clone())
            .wrap(Logger::default())
            .wrap(cors)
            .service(web::scope("/health").service(health))
            .service(
                web::scope("/api")
                    .service(web::scope("/auth").service(register).service(login))
                    .service(
                        web::scope("/posts")
                            .service(list_posts)
                            .service(get_post)
                            .service(
                                web::scope("")
                                    .wrap(JwtAuthMiddleware::new(jwt_service.clone()))
                                    .service(create_post)
                                    .service(update_post)
                                    .service(delete_post),
                            ),
                    ),
            )
    })
    .bind(("0.0.0.0", port))?
    .shutdown_timeout(30)
    .disable_signals()
    .run();

    let handle = server.handle();

    tokio::select! {
        res = server => {
            res?;
            info!("rest: server returned");
        }
        _ = shutdown.cancelled() => {
            info!("rest: graceful shutdown initiated");
            handle.stop(true).await;
        }
    }

    Ok(())
}

async fn run_grpc(
    port: u16,
    auth_service: Arc<AuthService>,
    post_service: Arc<PostService>,
    jwt_service: Arc<JwtService>,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
    let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);

    let auth_api = AuthApi::new(auth_service);
    let post_api = PostApi::new(post_service.clone());
    let post_editor_api = PostEditorApi::new(post_service);

    let interceptor = JwtInterceptor::new(jwt_service);

    info!(%addr, "grpc: starting server");

    Server::builder()
        .add_service(AuthServiceServer::new(auth_api))
        .add_service(PostServiceServer::new(post_api))
        .add_service(PostEditorServiceServer::with_interceptor(
            post_editor_api,
            interceptor,
        ))
        .serve_with_shutdown(addr, async move {
            shutdown.cancelled().await;
            info!("grpc: shutdown signal received");
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    info!("grpc: server stopped");
    Ok(())
}

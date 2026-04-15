use std::net::SocketAddr;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{error, info, warn};

use crate::application::auth_service::AuthService;
use crate::application::post_service::PostService;
use crate::data::post_repository::PostRepository;
use crate::data::user_repository::UserRepository;
use crate::domain::error::AppError;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::database::{create_pool, run_migrations};
use crate::infrastructure::jwt::JwtService;
use crate::infrastructure::logging::init_logging;
use crate::presentation::grpc::auth_service::AuthApi;
use crate::presentation::grpc::proto::blog::auth_service_server::AuthServiceServer;
use crate::presentation::rest::auth_http_handlers::{login, register};
use crate::presentation::rest::middleware::JwtAuthMiddleware;
use crate::presentation::rest::posts_http_handlers::{
    create_post, delete_post, get_post, list_posts, update_post,
};

mod application;
mod data;
mod domain;
mod infrastructure;
mod presentation;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging()?;
    info!("starting blog server");

    let config = AppConfig::from_env()?;

    let pool = create_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let jwt_service = Arc::new(JwtService::new(&config.jwt_secret));

    let user_repository = UserRepository::new(pool.clone());
    let post_repository = PostRepository::new(pool.clone());

    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        jwt_service.clone(),
    ));
    let post_service = Arc::new(PostService::new(post_repository.clone()));

    let shutdown = CancellationToken::new();
    let mut tasks: JoinSet<(&'static str, Result<(), AppError>)> = JoinSet::new();

    // REST
    tasks.spawn({
        let auth = auth_service.clone();
        let post = post_service.clone();
        let jwt = jwt_service.clone();
        let shutdown = shutdown.clone();
        async move { ("rest", run_rest(auth, post, jwt, shutdown).await) }
    });

    // gRPC
    tasks.spawn({
        let auth = auth_service.clone();
        let post = post_service.clone();
        let jwt = jwt_service.clone();
        let shutdown = shutdown.clone();
        async move { ("grpc", run_grpc(auth,post,jwt, shutdown).await) }
    });

    tokio::select! {
        _ = wait_for_signal() => {
            info!("shutdown signal received, stopping all servers");
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

    info!("blog server shut down");
    Ok(())
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received"),
            _ = sigint.recv()  => info!("SIGINT received"),
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        info!("ctrl-c received");
    }
}

async fn run_rest(
    auth_service: Arc<AuthService>,
    post_service: Arc<PostService>,
    jwt_service: Arc<JwtService>,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
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
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/auth")
                            .service(register)
                            .service(login),
                    )
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
        .bind(("127.0.0.1", 8080))?
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
    auth_service: Arc<AuthService>,
    post_service: Arc<PostService>,
    jwt_service: Arc<JwtService>,
    shutdown: CancellationToken,
) -> Result<(), AppError> {
    let addr: SocketAddr = "0.0.0.0:50051"
        .parse()
        .map_err(|e: std::net::AddrParseError| AppError::Config(e.to_string()))?;

    let auth_api = AuthApi::new(auth_service);
    //let post_api = PostApi::new(post_service, jwt_service);

    info!(%addr, "grpc: starting server");

    Server::builder()
        .add_service(AuthServiceServer::new(auth_api))
        //.add_service(PostServiceServer::new(post_api))
        .serve_with_shutdown(addr, async move {
            shutdown.cancelled().await;
            info!("grpc: shutdown signal received");
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    info!("grpc: server stopped");
    Ok(())
}
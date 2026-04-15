use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blog_server::app::run_app;
use blog_server::infrastructure::config::AppConfig;
use blog_server::presentation::grpc::proto::blog::{
    CreatePostRequest, DeletePostRequest, GetPostRequest, ListPostsRequest, LoginRequest,
    RegisterRequest, UpdatePostRequest, auth_service_client::AuthServiceClient,
    post_editor_service_client::PostEditorServiceClient, post_service_client::PostServiceClient,
};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::{Code, Request, transport::Channel};

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

fn bearer(token: &str) -> tonic::metadata::MetadataValue<tonic::metadata::Ascii> {
    format!("Bearer {token}")
        .parse()
        .expect("invalid bearer token")
}

fn with_auth<T>(token: &str, body: T) -> Request<T> {
    let mut req = Request::new(body);
    req.metadata_mut().insert("authorization", bearer(token));
    req
}

async fn wait_for_grpc_ready(addr: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

    loop {
        match Channel::from_shared(addr.to_string())
            .expect("invalid grpc addr")
            .connect()
            .await
        {
            Ok(_) => return,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("gRPC server did not become ready: {e}"),
        }
    }
}

struct SpawnedApp {
    grpc_addr: String,
    shutdown: CancellationToken,
    handle: JoinHandle<()>,
}

async fn spawn_app(database_url: String, http_port: u16, grpc_port: u16) -> SpawnedApp {
    let config = AppConfig::from_env()
        .expect("config")
        .with_database_url(database_url)
        .with_jwt_secret("test-secret".into());

    let shutdown = CancellationToken::new();
    let app_shutdown = shutdown.clone();

    let handle = tokio::spawn(async move {
        let result = run_app(config, http_port, grpc_port, app_shutdown).await;
        if let Err(e) = result {
            panic!("app failed: {:?}", e);
        }
    });

    let grpc_addr = format!("http://127.0.0.1:{grpc_port}");
    wait_for_grpc_ready(&grpc_addr).await;

    SpawnedApp {
        grpc_addr,
        shutdown,
        handle,
    }
}

#[tokio::test]
async fn grpc_blog_flow() {
    let postgres = Postgres::default()
        .start()
        .await
        .expect("failed to start postgres container");

    let db_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("failed to get mapped postgres port");

    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{db_port}/postgres");

    let app = spawn_app(database_url, 18080, 15051).await;

    let suffix = unique_suffix();
    let username = format!("alice_{suffix}");
    let email = format!("alice_{suffix}@example.com");
    let password = "Secret123".to_string();

    let other_username = format!("mallory_{suffix}");
    let other_email = format!("mallory_{suffix}@example.com");
    let other_password = "Hacker123".to_string();

    let mut auth = AuthServiceClient::connect(app.grpc_addr.clone())
        .await
        .expect("connect auth client");

    let mut post = PostServiceClient::connect(app.grpc_addr.clone())
        .await
        .expect("connect post client");

    let mut editor = PostEditorServiceClient::connect(app.grpc_addr.clone())
        .await
        .expect("connect editor client");

    let register = auth
        .register(RegisterRequest {
            username: username.clone(),
            email: email.clone(),
            password: password.clone(),
        })
        .await
        .expect("register alice failed")
        .into_inner();

    assert!(!register.token.is_empty(), "token is empty");
    let user = register.user.expect("user missing");
    assert!(user.id > 0, "user.id must be > 0");

    let mut auth_token = register.token;

    let err = auth
        .register(RegisterRequest {
            username: username.clone(),
            email: email.clone(),
            password: password.clone(),
        })
        .await
        .expect_err("expected duplicate register failure");
    assert_eq!(err.code(), Code::AlreadyExists);

    let login = auth
        .login(LoginRequest {
            username: username.clone(),
            password: password.clone(),
        })
        .await
        .expect("login alice failed")
        .into_inner();

    assert!(!login.token.is_empty(), "login token is empty");
    auth_token = login.token;

    let err = auth
        .login(LoginRequest {
            username: username.clone(),
            password: "totally_wrong".into(),
        })
        .await
        .expect_err("expected unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);

    let err = auth
        .login(LoginRequest {
            username: "ghost_user_does_not_exist".into(),
            password: "whatever".into(),
        })
        .await
        .expect_err("expected unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);

    let register_other = auth
        .register(RegisterRequest {
            username: other_username.clone(),
            email: other_email.clone(),
            password: other_password.clone(),
        })
        .await
        .expect("register mallory failed")
        .into_inner();

    assert!(!register_other.token.is_empty(), "mallory token is empty");

    let login_other = auth
        .login(LoginRequest {
            username: other_username,
            password: other_password,
        })
        .await
        .expect("login mallory failed")
        .into_inner();

    let other_token = login_other.token;
    assert!(!other_token.is_empty(), "mallory login token is empty");

    let list = post
        .list_posts(ListPostsRequest {
            limit: Some(0),
            offset: Some(0),
        })
        .await
        .expect("list posts failed")
        .into_inner();

    assert!(list.total >= 0);
    assert!(list.limit >= 0);
    assert!(list.offset >= 0);

    let list = post
        .list_posts(ListPostsRequest {
            limit: Some(5),
            offset: Some(0),
        })
        .await
        .expect("list posts paginated failed")
        .into_inner();

    assert_eq!(list.limit, 5);
    assert!(list.posts.len() <= 5);

    let err = post
        .get_post(GetPostRequest { id: 999_999 })
        .await
        .expect_err("expected not found");
    assert_eq!(err.code(), Code::NotFound);

    let err = editor
        .create_post(CreatePostRequest {
            title: "Anonymous attempt".into(),
            content: "Should fail".into(),
        })
        .await
        .expect_err("expected unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);

    let mut bad_req = Request::new(CreatePostRequest {
        title: "Invalid token attempt".into(),
        content: "Should fail".into(),
    });
    bad_req.metadata_mut().insert(
        "authorization",
        "Bearer not.a.real.jwt.token"
            .parse()
            .expect("invalid auth header"),
    );

    let err = editor
        .create_post(bad_req)
        .await
        .expect_err("expected invalid token failure");
    assert_eq!(err.code(), Code::Unauthenticated);

    let created = editor
        .create_post(with_auth(
            &auth_token,
            CreatePostRequest {
                title: "First post".into(),
                content: "Hello, world!".into(),
            },
        ))
        .await
        .expect("create post failed")
        .into_inner();

    assert!(created.id > 0, "post id must be > 0");
    assert_eq!(created.title, "First post");
    assert_eq!(created.content, "Hello, world!");
    assert!(created.author_id > 0, "author_id must be > 0");

    let post_id = created.id;

    let err = editor
        .create_post(with_auth(
            &auth_token,
            CreatePostRequest {
                title: "   ".into(),
                content: "some content".into(),
            },
        ))
        .await
        .expect_err("expected invalid argument");
    assert_eq!(err.code(), Code::InvalidArgument);

    let err = editor
        .create_post(with_auth(
            &auth_token,
            CreatePostRequest {
                title: "Title only".into(),
                content: "   ".into(),
            },
        ))
        .await
        .expect_err("expected invalid argument");
    assert_eq!(err.code(), Code::InvalidArgument);

    let fetched = post
        .get_post(GetPostRequest { id: post_id })
        .await
        .expect("get created post failed")
        .into_inner();

    assert_eq!(fetched.id, post_id);

    let updated = editor
        .update_post(with_auth(
            &auth_token,
            UpdatePostRequest {
                id: post_id,
                title: Some("First post (updated)".into()),
                content: None,
            },
        ))
        .await
        .expect("update post failed")
        .into_inner();

    assert_eq!(updated.title, "First post (updated)");
    assert_eq!(updated.content, "Hello, world!");

    let err = editor
        .update_post(with_auth(
            &auth_token,
            UpdatePostRequest {
                id: post_id,
                title: None,
                content: None,
            },
        ))
        .await
        .expect_err("expected invalid argument");
    assert_eq!(err.code(), Code::InvalidArgument);

    let err = editor
        .update_post(UpdatePostRequest {
            id: post_id,
            title: Some("Hijack attempt".into()),
            content: None,
        })
        .await
        .expect_err("expected unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);

    let err = editor
        .update_post(with_auth(
            &other_token,
            UpdatePostRequest {
                id: post_id,
                title: Some("Hacked by mallory".into()),
                content: None,
            },
        ))
        .await
        .expect_err("expected permission denied");
    assert_eq!(err.code(), Code::PermissionDenied);

    let err = editor
        .update_post(with_auth(
            &auth_token,
            UpdatePostRequest {
                id: 999_999,
                title: Some("ghost".into()),
                content: None,
            },
        ))
        .await
        .expect_err("expected not found");
    assert_eq!(err.code(), Code::NotFound);

    let err = editor
        .delete_post(DeletePostRequest { id: post_id })
        .await
        .expect_err("expected unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);

    let err = editor
        .delete_post(with_auth(&other_token, DeletePostRequest { id: post_id }))
        .await
        .expect_err("expected permission denied");
    assert_eq!(err.code(), Code::PermissionDenied);

    editor
        .delete_post(with_auth(&auth_token, DeletePostRequest { id: post_id }))
        .await
        .expect("delete post failed");

    let err = editor
        .delete_post(with_auth(&auth_token, DeletePostRequest { id: post_id }))
        .await
        .expect_err("expected not found");
    assert_eq!(err.code(), Code::NotFound);

    let err = post
        .get_post(GetPostRequest { id: post_id })
        .await
        .expect_err("expected deleted post not found");
    assert_eq!(err.code(), Code::NotFound);

    app.shutdown.cancel();
    let _ = app.handle.await;

    drop(postgres);
}

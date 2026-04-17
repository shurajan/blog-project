use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blog_client::error::ClientError;
use blog_client::{AuthToken, PostPatch, Transport, connect};
use blog_server::AppConfig;
use blog_server::app::run_app;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

fn test_config(database_url: String) -> AppConfig {
    AppConfig {
        database_url,
        jwt_secret: "test-secret".into(),
    }
}

async fn wait_for_http_ready(base_url: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        match connect(Transport::Http {
            base_url: base_url.to_string(),
        })
        .await
        {
            Ok(_) => return,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("HTTP server did not become ready: {e}"),
        }
    }
}

struct SpawnedApp {
    http_base_url: String,
    shutdown: CancellationToken,
    handle: JoinHandle<()>,
}

async fn spawn_app(database_url: String, http_port: u16, grpc_port: u16) -> SpawnedApp {
    let config = test_config(database_url);
    let shutdown = CancellationToken::new();
    let app_shutdown = shutdown.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = run_app(config, http_port, grpc_port, app_shutdown).await {
            panic!("app failed: {:?}", e);
        }
    });

    let http_base_url = format!("http://127.0.0.1:{http_port}");
    wait_for_http_ready(&http_base_url).await;

    SpawnedApp {
        http_base_url,
        shutdown,
        handle,
    }
}

fn is_unauthorized(e: &ClientError) -> bool {
    matches!(e, ClientError::Unauthorized)
}

fn is_forbidden(e: &ClientError) -> bool {
    matches!(e, ClientError::Forbidden)
}

fn is_not_found(e: &ClientError) -> bool {
    matches!(e, ClientError::NotFound)
}

fn is_conflict(e: &ClientError) -> bool {
    matches!(e, ClientError::Conflict)
}

fn is_invalid_argument(e: &ClientError) -> bool {
    matches!(e, ClientError::InvalidArgument(_))
}

#[tokio::test]
async fn http_client_flow() {
    let postgres = Postgres::default()
        .start()
        .await
        .expect("failed to start postgres container");

    let db_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("failed to get mapped postgres port");

    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{db_port}/postgres");

    let app = spawn_app(database_url, 19082, 16053).await;

    let client = connect(Transport::Http {
        base_url: app.http_base_url.clone(),
    })
    .await
    .expect("failed to connect blog client");

    let suffix = unique_suffix();
    let username = format!("alice_{suffix}");
    let email = format!("alice_{suffix}@example.com");
    let password = "Secret123";

    let other_username = format!("mallory_{suffix}");
    let other_email = format!("mallory_{suffix}@example.com");
    let other_password = "Hacker123";

    // --- register alice ---
    let (token, user_id) = client
        .register(&username, &email, password)
        .await
        .expect("register alice failed");

    assert!(!token.0.is_empty(), "token must not be empty");
    assert!(user_id > 0, "user_id must be > 0");

    // --- duplicate register ---
    let err = client
        .register(&username, &email, password)
        .await
        .expect_err("expected duplicate register to fail");
    assert!(is_conflict(&err), "expected Conflict, got: {err}");

    // --- login alice ---
    let auth_token = client
        .login(&username, password)
        .await
        .expect("login alice failed");

    assert!(!auth_token.0.is_empty(), "login token must not be empty");

    // --- login with wrong password ---
    let err = client
        .login(&username, "totally_wrong")
        .await
        .expect_err("expected unauthenticated");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- login with non-existent user ---
    let err = client
        .login("ghost_user_does_not_exist", "whatever")
        .await
        .expect_err("expected unauthenticated");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- register mallory ---
    let (other_token_reg, _) = client
        .register(&other_username, &other_email, other_password)
        .await
        .expect("register mallory failed");
    assert!(
        !other_token_reg.0.is_empty(),
        "mallory token must not be empty"
    );

    let other_token = client
        .login(&other_username, other_password)
        .await
        .expect("login mallory failed");
    assert!(
        !other_token.0.is_empty(),
        "mallory login token must not be empty"
    );

    // --- list posts (limit 0, clamped to 1 by server) ---
    let page = client
        .list_posts(Some(0), Some(0))
        .await
        .expect("list posts failed");
    assert!(page.total >= 0);
    assert!(page.limit >= 0);
    assert!(page.offset >= 0);

    // --- list posts (limit 5) ---
    let page = client
        .list_posts(Some(5), Some(0))
        .await
        .expect("list posts paginated failed");
    assert_eq!(page.limit, 5);
    assert!(page.posts.len() <= 5);

    // --- get non-existent post ---
    let err = client
        .get_post(999_999)
        .await
        .expect_err("expected not found");
    assert!(is_not_found(&err), "expected NotFound, got: {err}");

    // --- create post without token (empty) ---
    let err = client
        .create_post(
            &AuthToken(String::new()),
            "Anonymous attempt",
            "Should fail",
        )
        .await
        .expect_err("expected unauthenticated");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- create post with invalid token ---
    let err = client
        .create_post(
            &AuthToken("not.a.real.jwt.token".into()),
            "Invalid token attempt",
            "Should fail",
        )
        .await
        .expect_err("expected invalid token failure");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- create post successfully ---
    let created = client
        .create_post(&auth_token, "First post", "Hello, world!")
        .await
        .expect("create post failed");

    assert!(created.id > 0, "post id must be > 0");
    assert_eq!(created.title, "First post");
    assert_eq!(created.content, "Hello, world!");
    assert!(created.author_id > 0, "author_id must be > 0");

    let post_id = created.id;

    // --- create post with blank title ---
    let err = client
        .create_post(&auth_token, "   ", "some content")
        .await
        .expect_err("expected invalid argument");
    assert!(
        is_invalid_argument(&err),
        "expected InvalidArgument, got: {err}"
    );

    // --- create post with blank content ---
    let err = client
        .create_post(&auth_token, "Title only", "   ")
        .await
        .expect_err("expected invalid argument");
    assert!(
        is_invalid_argument(&err),
        "expected InvalidArgument, got: {err}"
    );

    // --- get created post ---
    let fetched = client.get_post(post_id).await.expect("get post failed");
    assert_eq!(fetched.id, post_id);
    assert_eq!(fetched.title, "First post");

    // --- update post (title only) ---
    let updated = client
        .update_post(
            &auth_token,
            post_id,
            PostPatch {
                title: Some("First post (updated)".into()),
                content: None,
            },
        )
        .await
        .expect("update post failed");
    assert_eq!(updated.title, "First post (updated)");
    assert_eq!(updated.content, "Hello, world!");

    // --- update post with no fields ---
    let err = client
        .update_post(
            &auth_token,
            post_id,
            PostPatch {
                title: None,
                content: None,
            },
        )
        .await
        .expect_err("expected invalid argument");
    assert!(
        is_invalid_argument(&err),
        "expected InvalidArgument, got: {err}"
    );

    // --- update post without token ---
    let err = client
        .update_post(
            &AuthToken(String::new()),
            post_id,
            PostPatch {
                title: Some("Hijack attempt".into()),
                content: None,
            },
        )
        .await
        .expect_err("expected unauthenticated");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- update post by another user ---
    let err = client
        .update_post(
            &other_token,
            post_id,
            PostPatch {
                title: Some("Hacked by mallory".into()),
                content: None,
            },
        )
        .await
        .expect_err("expected permission denied");
    assert!(is_forbidden(&err), "expected Forbidden, got: {err}");

    // --- update non-existent post ---
    let err = client
        .update_post(
            &auth_token,
            999_999,
            PostPatch {
                title: Some("ghost".into()),
                content: None,
            },
        )
        .await
        .expect_err("expected not found");
    assert!(is_not_found(&err), "expected NotFound, got: {err}");

    // --- delete post without token ---
    let err = client
        .delete_post(&AuthToken(String::new()), post_id)
        .await
        .expect_err("expected unauthenticated");
    assert!(is_unauthorized(&err), "expected Unauthorized, got: {err}");

    // --- delete post by another user ---
    let err = client
        .delete_post(&other_token, post_id)
        .await
        .expect_err("expected permission denied");
    assert!(is_forbidden(&err), "expected Forbidden, got: {err}");

    // --- delete post successfully ---
    client
        .delete_post(&auth_token, post_id)
        .await
        .expect("delete post failed");

    // --- delete already-deleted post ---
    let err = client
        .delete_post(&auth_token, post_id)
        .await
        .expect_err("expected not found after deletion");
    assert!(is_not_found(&err), "expected NotFound, got: {err}");

    // --- get deleted post ---
    let err = client
        .get_post(post_id)
        .await
        .expect_err("expected deleted post not found");
    assert!(is_not_found(&err), "expected NotFound, got: {err}");

    app.shutdown.cancel();
    let _ = app.handle.await;

    drop(postgres);
}

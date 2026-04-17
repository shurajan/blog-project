use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blog_server::AppConfig;
use blog_server::app::run_app;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

fn test_config(database_url: String) -> AppConfig {
    AppConfig {
        database_url,
        jwt_secret: "test-secret".into(),
    }
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis()
}

async fn wait_for_http_ready(base_url: &str) {
    let client = Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

    loop {
        match client.get(format!("{base_url}/api/posts")).send().await {
            Ok(_) => return,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("HTTP server did not become ready: {e}"),
        }
    }
}

struct SpawnedApp {
    base_url: String,
    shutdown: CancellationToken,
    handle: JoinHandle<()>,
}

async fn spawn_app(database_url: String, http_port: u16, grpc_port: u16) -> SpawnedApp {
    let config = test_config(database_url.clone());

    let shutdown = CancellationToken::new();
    let app_shutdown = shutdown.clone();

    let handle = tokio::spawn(async move {
        let result = run_app(config, http_port, grpc_port, app_shutdown).await;
        if let Err(e) = result {
            panic!("app failed: {:?}", e);
        }
    });

    let base_url = format!("http://127.0.0.1:{http_port}");
    wait_for_http_ready(&base_url).await;

    SpawnedApp {
        base_url,
        shutdown,
        handle,
    }
}

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

#[tokio::test]
async fn rest_blog_flow() {
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
    let client = Client::new();

    let suffix = unique_suffix();
    let username = format!("alice_{suffix}");
    let email = format!("alice_{suffix}@example.com");
    let password = "Secret123".to_string();

    let other_username = format!("mallory_{suffix}");
    let other_email = format!("mallory_{suffix}@example.com");
    let other_password = "Hacker123".to_string();

    // 1. Register alice — happy path
    let resp = client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("register alice request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.expect("invalid register alice json");

    let _auth_token = body["token"]
        .as_str()
        .expect("no token in register response")
        .to_string();
    let user_id = body["user"]["id"]
        .as_i64()
        .expect("no user.id in register response");
    assert!(user_id > 0);

    // 2. Register duplicate — 409
    let resp = client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "username": body["user"]["username"].as_str().unwrap_or("unused"),
            "email": body["user"]["email"].as_str().unwrap_or("unused@example.com"),
            "password": "Secret123",
        }))
        .send()
        .await
        .expect("duplicate register request failed");

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // 3. Login alice — 200
    let resp = client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "username": body["user"]["username"].as_str().unwrap_or(""),
            "password": "Secret123",
        }))
        .send()
        .await
        .expect("login alice request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("invalid login alice json");
    let auth_token = body["token"]
        .as_str()
        .expect("no token in login response")
        .to_string();

    // 4. Login wrong password — 401
    let resp = client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "username": format!("alice_{suffix}"),
            "password": "totally_wrong",
        }))
        .send()
        .await
        .expect("wrong password request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 5. Login unknown user — 401
    let resp = client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "username": "ghost_user_does_not_exist",
            "password": "whatever",
        }))
        .send()
        .await
        .expect("unknown user login request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 6. Register missing field — 400
    let resp = client
        .post(format!("{}/api/auth/register", app.base_url))
        .header("Content-Type", "application/json")
        .body(r#"{"username":"bob","email":"bob@example.com"}"#)
        .send()
        .await
        .expect("missing field request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 7. Register malformed JSON — 400
    let resp = client
        .post(format!("{}/api/auth/register", app.base_url))
        .header("Content-Type", "application/json")
        .body(r#"{"username":"bob"}"#)
        .send()
        .await
        .expect("malformed json request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 8. Register mallory
    let resp = client
        .post(format!("{}/api/auth/register", app.base_url))
        .json(&json!({
            "username": other_username,
            "email": other_email,
            "password": other_password,
        }))
        .send()
        .await
        .expect("register mallory request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.expect("invalid register mallory json");
    let maybe_other_token = body["token"].as_str().map(ToOwned::to_owned);

    // 9. Login mallory
    let resp = client
        .post(format!("{}/api/auth/login", app.base_url))
        .json(&json!({
            "username": format!("mallory_{suffix}"),
            "password": "Hacker123",
        }))
        .send()
        .await
        .expect("login mallory request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("invalid login mallory json");
    let other_token = body["token"]
        .as_str()
        .map(ToOwned::to_owned)
        .or(maybe_other_token)
        .expect("no token for mallory");

    // 10. List posts
    let resp = client
        .get(format!("{}/api/posts", app.base_url))
        .send()
        .await
        .expect("list posts request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("invalid list posts json");
    assert!(body["posts"].is_array());
    assert!(body["total"].is_number());
    assert!(body["limit"].is_number());
    assert!(body["offset"].is_number());

    // 11. List posts with pagination
    let resp = client
        .get(format!("{}/api/posts?limit=5&offset=0", app.base_url))
        .send()
        .await
        .expect("paginated list posts request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp
        .json()
        .await
        .expect("invalid paginated list posts json");
    assert_eq!(body["limit"].as_i64().expect("limit missing"), 5);
    assert!(body["posts"].as_array().expect("posts not array").len() <= 5);

    // 12. Get non-existent post
    let resp = client
        .get(format!("{}/api/posts/999999", app.base_url))
        .send()
        .await
        .expect("get nonexistent post request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 13. Create post without token
    let resp = client
        .post(format!("{}/api/posts", app.base_url))
        .json(&json!({
            "title": "Anonymous attempt",
            "content": "Should fail",
        }))
        .send()
        .await
        .expect("create without token request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 14. Create post with garbage token
    let resp = client
        .post(format!("{}/api/posts", app.base_url))
        .header("Authorization", "Bearer not.a.real.jwt.token")
        .json(&json!({
            "title": "Invalid token attempt",
            "content": "Should fail",
        }))
        .send()
        .await
        .expect("create with invalid token request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 15. Create post — happy path
    let resp = client
        .post(format!("{}/api/posts", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .json(&json!({
            "title": "First post",
            "content": "Hello, world!",
        }))
        .send()
        .await
        .expect("create post request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.expect("invalid create post json");
    let post_id = body["id"].as_i64().expect("no id");
    assert_eq!(body["title"].as_str().unwrap_or_default(), "First post");
    assert_eq!(
        body["content"].as_str().unwrap_or_default(),
        "Hello, world!"
    );
    assert!(body["author_id"].as_i64().expect("no author_id") > 0);

    // 16. Create post — empty title
    let resp = client
        .post(format!("{}/api/posts", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .json(&json!({
            "title": "   ",
            "content": "content",
        }))
        .send()
        .await
        .expect("create empty title request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 17. Create post — missing field
    let resp = client
        .post(format!("{}/api/posts", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .header("Content-Type", "application/json")
        .body(r#"{"title":"no content"}"#)
        .send()
        .await
        .expect("create missing field request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 18. Get created post
    let resp = client
        .get(format!("{}/api/posts/{post_id}", app.base_url))
        .send()
        .await
        .expect("get created post request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("invalid get created post json");
    assert_eq!(body["id"].as_i64().expect("no id"), post_id);

    // 19. Update post — happy path
    let resp = client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .json(&json!({
            "title": "First post (updated)"
        }))
        .send()
        .await
        .expect("update post request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("invalid update post json");
    assert_eq!(
        body["title"].as_str().unwrap_or_default(),
        "First post (updated)"
    );
    assert_eq!(
        body["content"].as_str().unwrap_or_default(),
        "Hello, world!"
    );

    // 20. Update post — empty patch
    let resp = client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .json(&json!({}))
        .send()
        .await
        .expect("empty patch request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 21. Update post — without token
    let resp = client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .json(&json!({
            "title": "Hijack attempt"
        }))
        .send()
        .await
        .expect("update without token request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 22. Update post — wrong owner
    let resp = client
        .put(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&other_token))
        .json(&json!({
            "title": "Hacked by mallory"
        }))
        .send()
        .await
        .expect("update wrong owner request failed");

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 23. Update non-existent post
    let resp = client
        .put(format!("{}/api/posts/999999", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .json(&json!({
            "title": "ghost"
        }))
        .send()
        .await
        .expect("update nonexistent post request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 24. Delete post — without token
    let resp = client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .send()
        .await
        .expect("delete without token request failed");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 25. Delete post — wrong owner
    let resp = client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&other_token))
        .send()
        .await
        .expect("delete wrong owner request failed");

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 26. Delete post — happy path
    let resp = client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .send()
        .await
        .expect("delete post request failed");

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 27. Delete post — already deleted
    let resp = client
        .delete(format!("{}/api/posts/{post_id}", app.base_url))
        .header("Authorization", auth_header(&auth_token))
        .send()
        .await
        .expect("delete already deleted request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 28. Get deleted post
    let resp = client
        .get(format!("{}/api/posts/{post_id}", app.base_url))
        .send()
        .await
        .expect("get deleted post request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    app.shutdown.cancel();
    let _ = app.handle.await;

    drop(postgres);
}

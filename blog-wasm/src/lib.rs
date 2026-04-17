use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement, Window};

const API_BASE: &str = "http://localhost:3000";
const TOKEN_KEY: &str = "blog_jwt_token";
const USER_ID_KEY: &str = "blog_user_id";

// ── Models ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Post {
    id: i64,
    author_id: i64,
    title: String,
    content: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct PostPage {
    posts: Vec<Post>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    user: UserInfo,
    token: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    id: i64,
    username: String,
}

#[derive(Debug, Serialize)]
struct RegisterRequest<'a> {
    username: &'a str,
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct CreatePostRequest<'a> {
    title: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct UpdatePostRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    spawn_local(async {
        render_app().await;
    });
}

async fn render_app() {
    let token: Option<String> = LocalStorage::get(TOKEN_KEY).ok();
    let user_id: Option<i64> = LocalStorage::get(USER_ID_KEY).ok();

    update_auth_ui(token.is_some());

    let posts = fetch_posts().await;
    render_posts(&posts, user_id);

    setup_register_form();
    setup_login_form();
    setup_create_post_form();
    setup_logout_button();
}

// ── Auth helpers ──────────────────────────────────────────────────────────────

fn current_token() -> Option<String> {
    LocalStorage::get(TOKEN_KEY).ok()
}

fn current_user_id() -> Option<i64> {
    LocalStorage::get(USER_ID_KEY).ok()
}

fn save_auth(token: &str, user_id: i64) {
    if let Err(err) = LocalStorage::set(TOKEN_KEY, token) {
        log_str(&format!("Failed to save token: {:?}", err));
    }
    if let Err(err) = LocalStorage::set(USER_ID_KEY, user_id) {
        log_str(&format!("Failed to save user id: {:?}", err));
    }
}

fn clear_auth() {
    LocalStorage::delete(TOKEN_KEY);
    LocalStorage::delete(USER_ID_KEY);
}

fn update_auth_ui(logged_in: bool) {
    let Some(doc) = document() else {
        return;
    };

    set_display(
        &doc,
        "auth-section",
        if logged_in { "none" } else { "block" },
    );
    set_display(
        &doc,
        "create-post-section",
        if logged_in { "block" } else { "none" },
    );
    set_display(
        &doc,
        "logout-section",
        if logged_in { "block" } else { "none" },
    );

    if logged_in {
        set_text(&doc, "auth-status", "Status: Logged in");
    } else {
        set_text(&doc, "auth-status", "Status: Not logged in");
    }
}

// ── API ───────────────────────────────────────────────────────────────────────

async fn fetch_posts() -> Vec<Post> {
    let url = format!("{}/api/posts?limit=100&offset=0", API_BASE);
    match Request::get(&url).send().await {
        Ok(resp) if resp.ok() => match resp.json::<PostPage>().await {
            Ok(page) => page.posts,
            Err(err) => {
                log_str(&format!("Failed to parse posts response: {}", err));
                vec![]
            }
        },
        Ok(resp) => {
            log_str(&format!(
                "Fetch posts failed with status: {}",
                resp.status()
            ));
            vec![]
        }
        Err(err) => {
            log_str(&format!("Fetch posts request failed: {}", err));
            vec![]
        }
    }
}

async fn api_register(username: &str, email: &str, password: &str) -> Result<AuthResponse, String> {
    let body = serde_json::to_string(&RegisterRequest {
        username,
        email,
        password,
    })
    .map_err(|e| e.to_string())?;

    let resp = Request::post(&format!("{}/api/auth/register", API_BASE))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<AuthResponse>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(parse_error_text(&text))
    }
}

async fn api_login(username: &str, password: &str) -> Result<AuthResponse, String> {
    let body =
        serde_json::to_string(&LoginRequest { username, password }).map_err(|e| e.to_string())?;

    let resp = Request::post(&format!("{}/api/auth/login", API_BASE))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<AuthResponse>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(parse_error_text(&text))
    }
}

async fn api_create_post(token: &str, title: &str, content: &str) -> Result<Post, String> {
    let body =
        serde_json::to_string(&CreatePostRequest { title, content }).map_err(|e| e.to_string())?;

    let resp = Request::post(&format!("{}/api/posts", API_BASE))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<Post>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(parse_error_text(&text))
    }
}

async fn api_update_post(
    token: &str,
    id: i64,
    title: Option<String>,
    content: Option<String>,
) -> Result<Post, String> {
    let body =
        serde_json::to_string(&UpdatePostRequest { title, content }).map_err(|e| e.to_string())?;

    let resp = Request::put(&format!("{}/api/posts/{}", API_BASE, id))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<Post>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(parse_error_text(&text))
    }
}

async fn api_delete_post(token: &str, id: i64) -> Result<(), String> {
    let resp = Request::delete(&format!("{}/api/posts/{}", API_BASE, id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() || resp.status() == 204 {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(parse_error_text(&text))
    }
}

fn parse_error_text(text: &str) -> String {
    #[derive(Deserialize)]
    struct ErrBody {
        error: String,
    }

    serde_json::from_str::<ErrBody>(text)
        .map(|b| b.error)
        .unwrap_or_else(|_| text.to_string())
}

// ── DOM rendering ─────────────────────────────────────────────────────────────

fn render_posts(posts: &[Post], current_user: Option<i64>) {
    let Some(doc) = document() else {
        return;
    };

    let Some(container) = doc.get_element_by_id("posts-list") else {
        return;
    };

    container.set_inner_html("");

    if posts.is_empty() {
        container.set_inner_html("<p class=\"empty\">No posts yet.</p>");
        return;
    }

    for post in posts {
        let card = match doc.create_element("div") {
            Ok(el) => el,
            Err(err) => {
                log_js(&err);
                continue;
            }
        };

        card.set_class_name("post-card");

        if let Err(err) = card.set_attribute("data-id", &post.id.to_string()) {
            log_js(&err);
        }

        let is_author = current_user == Some(post.author_id);

        let actions = if is_author {
            format!(
                r#"<div class="post-actions">
                    <button class="btn-edit" data-id="{id}" data-title="{title}" data-content="{content}">Edit</button>
                    <button class="btn-delete" data-id="{id}">Delete</button>
                </div>"#,
                id = post.id,
                title = html_escape_attr(&post.title),
                content = html_escape_attr(&post.content),
            )
        } else {
            String::new()
        };

        card.set_inner_html(&format!(
            r#"<h3 class="post-title">{title}</h3>
               <p class="post-content">{content}</p>
               <span class="post-meta">Post #{id}</span>
               {actions}"#,
            title = html_escape_html(&post.title),
            content = html_escape_html(&post.content),
            id = post.id,
            actions = actions,
        ));

        if let Err(err) = container.append_child(&card) {
            log_js(&err);
        }
    }

    setup_post_action_buttons();
}

fn setup_post_action_buttons() {
    let Some(doc) = document() else {
        return;
    };

    let edits = match doc.query_selector_all(".btn-edit") {
        Ok(list) => list,
        Err(err) => {
            log_js(&err);
            return;
        }
    };

    for i in 0..edits.length() {
        let Some(node) = edits.item(i) else {
            continue;
        };

        let Ok(btn) = node.dyn_into::<Element>() else {
            continue;
        };

        let id = btn
            .get_attribute("data-id")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let title = btn.get_attribute("data-title").unwrap_or_default();
        let content = btn.get_attribute("data-content").unwrap_or_default();
        let doc2 = doc.clone();

        let cb = Closure::<dyn Fn()>::new(move || {
            show_edit_modal(&doc2, id, &title, &content);
        });

        if let Some(html_btn) = btn.dyn_ref::<HtmlElement>() {
            html_btn.set_onclick(Some(cb.as_ref().unchecked_ref()));
        }

        cb.forget();
    }

    let deletes = match doc.query_selector_all(".btn-delete") {
        Ok(list) => list,
        Err(err) => {
            log_js(&err);
            return;
        }
    };

    for i in 0..deletes.length() {
        let Some(node) = deletes.item(i) else {
            continue;
        };

        let Ok(btn) = node.dyn_into::<Element>() else {
            continue;
        };

        let id = btn
            .get_attribute("data-id")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let cb = Closure::<dyn Fn()>::new(move || {
            let Some(token) = current_token() else {
                return;
            };

            spawn_local(async move {
                match api_delete_post(&token, id).await {
                    Ok(_) => {
                        let posts = fetch_posts().await;
                        render_posts(&posts, current_user_id());
                        show_message("Post deleted.", false);
                    }
                    Err(e) => show_message(&e, true),
                }
            });
        });

        if let Some(html_btn) = btn.dyn_ref::<HtmlElement>() {
            html_btn.set_onclick(Some(cb.as_ref().unchecked_ref()));
        }

        cb.forget();
    }
}

fn show_edit_modal(doc: &Document, id: i64, title: &str, content: &str) {
    let Some(modal) = doc.get_element_by_id("edit-modal") else {
        return;
    };

    set_input_value(doc, "edit-title", title);
    set_textarea_value(doc, "edit-content", content);

    if let Err(err) = modal.set_attribute("data-post-id", &id.to_string()) {
        log_js(&err);
    }

    if let Some(html_modal) = modal.dyn_ref::<HtmlElement>() {
        if let Err(err) = html_modal.style().set_property("display", "flex") {
            log_js(&err);
        }
    }
}

// ── Form setup ────────────────────────────────────────────────────────────────

fn setup_register_form() {
    let Some(doc) = document() else {
        return;
    };

    let Some(form) = doc.get_element_by_id("register-form") else {
        return;
    };

    let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| {
        e.prevent_default();

        let Some(doc) = document() else {
            show_message("Document is not available.", true);
            return;
        };

        let username = get_input_value(&doc, "reg-username");
        let email = get_input_value(&doc, "reg-email");
        let password = get_input_value(&doc, "reg-password");

        if username.is_empty() || email.is_empty() || password.is_empty() {
            show_message("All fields are required.", true);
            return;
        }

        spawn_local(async move {
            match api_register(&username, &email, &password).await {
                Ok(resp) => {
                    save_auth(&resp.token, resp.user.id);
                    update_auth_ui(true);
                    show_message(&format!("Registered as {}!", resp.user.username), false);
                    let posts = fetch_posts().await;
                    render_posts(&posts, Some(resp.user.id));
                }
                Err(e) => show_message(&e, true),
            }
        });
    });

    if let Err(err) = form.add_event_listener_with_callback("submit", cb.as_ref().unchecked_ref()) {
        log_js(&err);
        return;
    }

    cb.forget();
}

fn setup_login_form() {
    let Some(doc) = document() else {
        return;
    };

    let Some(form) = doc.get_element_by_id("login-form") else {
        return;
    };

    let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| {
        e.prevent_default();

        let Some(doc) = document() else {
            show_message("Document is not available.", true);
            return;
        };

        let username = get_input_value(&doc, "login-username");
        let password = get_input_value(&doc, "login-password");

        if username.is_empty() || password.is_empty() {
            show_message("Username and password are required.", true);
            return;
        }

        spawn_local(async move {
            match api_login(&username, &password).await {
                Ok(resp) => {
                    save_auth(&resp.token, resp.user.id);
                    update_auth_ui(true);
                    show_message(&format!("Welcome back, {}!", resp.user.username), false);
                    let posts = fetch_posts().await;
                    render_posts(&posts, Some(resp.user.id));
                }
                Err(e) => show_message(&e, true),
            }
        });
    });

    if let Err(err) = form.add_event_listener_with_callback("submit", cb.as_ref().unchecked_ref()) {
        log_js(&err);
        return;
    }

    cb.forget();
}

fn setup_create_post_form() {
    let Some(doc) = document() else {
        return;
    };

    let Some(form) = doc.get_element_by_id("create-post-form") else {
        return;
    };

    let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |e: web_sys::Event| {
        e.prevent_default();

        let Some(doc) = document() else {
            show_message("Document is not available.", true);
            return;
        };

        let title = get_input_value(&doc, "post-title");
        let content = get_textarea_value(&doc, "post-content");

        if title.is_empty() || content.is_empty() {
            show_message("Title and content are required.", true);
            return;
        }

        let Some(token) = current_token() else {
            show_message("You must be logged in.", true);
            return;
        };

        spawn_local(async move {
            match api_create_post(&token, &title, &content).await {
                Ok(_) => {
                    if let Some(doc) = document() {
                        set_input_value(&doc, "post-title", "");
                        set_textarea_value(&doc, "post-content", "");
                    }
                    show_message("Post created!", false);
                    let posts = fetch_posts().await;
                    render_posts(&posts, current_user_id());
                }
                Err(e) => show_message(&e, true),
            }
        });
    });

    if let Err(err) = form.add_event_listener_with_callback("submit", cb.as_ref().unchecked_ref()) {
        log_js(&err);
        return;
    }

    cb.forget();

    setup_edit_modal();
}

fn setup_edit_modal() {
    let Some(doc) = document() else {
        return;
    };

    if let Some(close_btn) = doc.get_element_by_id("edit-modal-close") {
        let cb = Closure::<dyn Fn()>::new(move || {
            let Some(doc) = document() else {
                return;
            };

            if let Some(modal) = doc.get_element_by_id("edit-modal") {
                if let Some(html_modal) = modal.dyn_ref::<HtmlElement>() {
                    if let Err(err) = html_modal.style().set_property("display", "none") {
                        log_js(&err);
                    }
                }
            }
        });

        if let Some(html_btn) = close_btn.dyn_ref::<HtmlElement>() {
            html_btn.set_onclick(Some(cb.as_ref().unchecked_ref()));
        }

        cb.forget();
    }

    if let Some(save_btn) = doc.get_element_by_id("edit-modal-save") {
        let cb = Closure::<dyn Fn()>::new(move || {
            let Some(doc) = document() else {
                show_message("Document is not available.", true);
                return;
            };

            let title = get_input_value(&doc, "edit-title");
            let content = get_textarea_value(&doc, "edit-content");

            if title.is_empty() && content.is_empty() {
                show_message("At least one field is required.", true);
                return;
            }

            let id = doc
                .get_element_by_id("edit-modal")
                .and_then(|el| el.get_attribute("data-post-id"))
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            if id == 0 {
                show_message("Invalid post id.", true);
                return;
            }

            let Some(token) = current_token() else {
                show_message("You must be logged in.", true);
                return;
            };

            spawn_local(async move {
                let t = if title.is_empty() { None } else { Some(title) };
                let c = if content.is_empty() {
                    None
                } else {
                    Some(content)
                };

                match api_update_post(&token, id, t, c).await {
                    Ok(_) => {
                        if let Some(doc) = document() {
                            if let Some(modal) = doc.get_element_by_id("edit-modal") {
                                if let Some(html_modal) = modal.dyn_ref::<HtmlElement>() {
                                    if let Err(err) =
                                        html_modal.style().set_property("display", "none")
                                    {
                                        log_js(&err);
                                    }
                                }
                            }
                        }

                        show_message("Post updated!", false);
                        let posts = fetch_posts().await;
                        render_posts(&posts, current_user_id());
                    }
                    Err(e) => show_message(&e, true),
                }
            });
        });

        if let Some(html_btn) = save_btn.dyn_ref::<HtmlElement>() {
            html_btn.set_onclick(Some(cb.as_ref().unchecked_ref()));
        }

        cb.forget();
    }
}

fn setup_logout_button() {
    let Some(doc) = document() else {
        return;
    };

    let Some(btn) = doc.get_element_by_id("logout-btn") else {
        return;
    };

    let cb = Closure::<dyn Fn()>::new(move || {
        clear_auth();
        update_auth_ui(false);
        show_message("Logged out.", false);

        spawn_local(async {
            let posts = fetch_posts().await;
            render_posts(&posts, None);
        });
    });

    if let Some(html_btn) = btn.dyn_ref::<HtmlElement>() {
        html_btn.set_onclick(Some(cb.as_ref().unchecked_ref()));
    }

    cb.forget();
}

// ── DOM helpers ───────────────────────────────────────────────────────────────

fn window() -> Option<Window> {
    web_sys::window()
}

fn document() -> Option<Document> {
    window()?.document()
}

fn set_display(doc: &Document, id: &str, display: &str) {
    let Some(el) = doc.get_element_by_id(id) else {
        return;
    };

    let Some(html_el) = el.dyn_ref::<HtmlElement>() else {
        return;
    };

    if let Err(err) = html_el.style().set_property("display", display) {
        log_js(&err);
    }
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Some(el) = doc.get_element_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn get_input_value(doc: &Document, id: &str) -> String {
    doc.get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|el| el.value().trim().to_string())
        .unwrap_or_default()
}

fn set_input_value(doc: &Document, id: &str, val: &str) {
    if let Some(el) = doc
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
    {
        el.set_value(val);
    }
}

fn get_textarea_value(doc: &Document, id: &str) -> String {
    doc.get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|el| el.value().trim().to_string())
        .unwrap_or_default()
}

fn set_textarea_value(doc: &Document, id: &str, val: &str) {
    if let Some(el) = doc
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
    {
        el.set_value(val);
    }
}

fn show_message(msg: &str, is_error: bool) {
    let Some(doc) = document() else {
        return;
    };

    let Some(el) = doc.get_element_by_id("message-box") else {
        return;
    };

    el.set_text_content(Some(msg));
    el.set_class_name(if is_error {
        "message error"
    } else {
        "message success"
    });

    let el_clone = el.clone();
    let cb = Closure::once(move || {
        el_clone.set_text_content(Some(""));
        el_clone.set_class_name("message");
    });

    if let Some(win) = window() {
        if let Err(err) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            4000,
        ) {
            log_js(&err);
        }
    }

    cb.forget();
}

// ── Escaping ──────────────────────────────────────────────────────────────────

fn html_escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Logging ───────────────────────────────────────────────────────────────────

fn log_js(value: &JsValue) {
    web_sys::console::error_1(value);
}

fn log_str(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

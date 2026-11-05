//! Plugins through the HTTP API: install, list, switch, serve, store, remove.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use bacnetio::server::{self, AppState};
use serde_json::{Value, json};
use tower::ServiceExt;

const PLUGIN: &str = r#"<!doctype html><title>Test</title>
<script type="application/json" id="bacnetio-plugin">
{"id":"test","name":"Test plugin","version":"1.0.0","api":1,"description":"For the tests"}
</script><p>hello</p>"#;

fn app(name: &str) -> Router {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("bacnetio-plugins-{name}-{}", std::process::id()));
    // left over from an aborted run
    let _ = std::fs::remove_dir_all(&dir);
    server::router(Arc::new(AppState::with_data_dir(dir).unwrap()))
}

async fn call(
    app: &Router,
    method: Method,
    path: &str,
    body: &str,
) -> (StatusCode, Value, Option<String>) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let policy = response
        .headers()
        .get(header::CONTENT_SECURITY_POLICY)
        .map(|value| value.to_str().unwrap().to_string());
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
    (status, body, policy)
}

#[tokio::test]
async fn a_plugin_is_installed_switched_and_removed() {
    let app = app("life");
    let (status, list, _) = call(&app, Method::GET, "/api/plugins", "").await;
    assert_eq!((status, list), (StatusCode::OK, json!([])));

    let (status, plugin, _) = call(&app, Method::POST, "/api/plugins", PLUGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (plugin["id"].as_str(), plugin["on"].as_bool()),
        (Some("test"), Some(true))
    );
    let (_, list, _) = call(&app, Method::GET, "/api/plugins", "").await;
    assert_eq!(list[0]["name"], "Test plugin");

    // the same id again only when the user agreed to replace it
    let (status, body, _) = call(&app, Method::POST, "/api/plugins", PLUGIN).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body["error"],
        "plugin Test plugin 1.0.0 is installed already"
    );
    let newer = PLUGIN.replace("1.0.0", "1.1.0");
    let (status, body, _) = call(&app, Method::POST, "/api/plugins?replace=true", &newer).await;
    assert_eq!(
        (status, body["version"].as_str()),
        (StatusCode::OK, Some("1.1.0"))
    );

    // the page comes in a sandbox that allows its own scripts and nothing else
    let (status, page, policy) = call(&app, Method::GET, "/api/plugins/test/page", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.as_str().unwrap().contains("hello"));
    let policy = policy.unwrap();
    assert!(policy.starts_with("sandbox allow-scripts;"), "{policy}");
    assert!(!policy.contains("allow-same-origin"));

    let (status, body, _) = call(
        &app,
        Method::POST,
        "/api/plugins/test/switch",
        r#"{"on":false}"#,
    )
    .await;
    assert_eq!(
        (status, body["on"].as_bool()),
        (StatusCode::OK, Some(false))
    );
    let (status, _, _) = call(&app, Method::GET, "/api/plugins/test/page", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    call(
        &app,
        Method::POST,
        "/api/plugins/test/switch",
        r#"{"on":true}"#,
    )
    .await;

    // the plugin's own store
    let (_, kept, _) = call(&app, Method::GET, "/api/plugins/test/store", "").await;
    assert_eq!(kept, Value::Null);
    call(
        &app,
        Method::POST,
        "/api/plugins/test/store",
        r#"{"checked":3}"#,
    )
    .await;
    let (_, kept, _) = call(&app, Method::GET, "/api/plugins/test/store", "").await;
    assert_eq!(kept, json!({ "checked": 3 }));

    let (status, _, _) = call(&app, Method::POST, "/api/plugins/test/remove", "").await;
    assert_eq!(status, StatusCode::OK);
    let (_, list, _) = call(&app, Method::GET, "/api/plugins", "").await;
    assert_eq!(list, json!([]));
    let (status, _, _) = call(&app, Method::GET, "/api/plugins/test/store", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn what_is_not_a_plugin_is_refused() {
    let app = app("refused");
    let (status, body, _) = call(&app, Method::POST, "/api/plugins", "<html>a page</html>").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"],
        "not a bacnetio plugin: the description is missing"
    );
    // an id from the address never reaches the file system unchecked
    let (status, _, _) = call(&app, Method::GET, "/api/plugins/..%2Foff/page", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn without_a_data_folder_there_are_no_plugins() {
    let app = server::router(Arc::new(AppState::default()));
    let (status, body, _) = call(&app, Method::GET, "/api/plugins", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "plugins need a data folder");
}

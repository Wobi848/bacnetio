//! The HTTP API without network: answers that do not need a BACnet connection.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use bacnetio::server::{self, AppState};
use serde_json::Value;
use tower::ServiceExt;

async fn call(method: Method, path: &str, body: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = server::router(Arc::new(AppState::default()))
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn adapters_are_listed() {
    let (status, body) = call(Method::GET, "/api/adapters", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}

#[tokio::test]
async fn scan_is_idle_at_start() {
    let (status, body) = call(Method::GET, "/api/scan", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "idle");
}

#[tokio::test]
async fn reading_before_a_scan_asks_for_one() {
    let (status, body) = call(Method::GET, "/api/devices/1/points", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "search for devices first");
}

/// The plant view comes from the last reading's object list, so it needs that reading.
#[tokio::test]
async fn the_plant_view_asks_for_the_points_first() {
    let (status, body) = call(Method::GET, "/api/devices/1/structure", "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "read the device's points first");
}

#[tokio::test]
async fn no_last_reading_before_one() {
    let (status, body) = call(Method::GET, "/api/devices/1/points/last", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

#[tokio::test]
async fn bad_input_is_refused_in_words() {
    let path = "/api/devices/1/objects/1/1/write";
    let (status, body) = call(Method::POST, path, r#"{"value":"warm","priority":8}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "\"warm\" is not a number");

    let (status, body) = call(Method::POST, path, r#"{"value":"1","priority":17}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "priority 17 is not between 1 and 16");

    let (status, _) = call(Method::POST, "/api/scan", r#"{"bbmd":"nowhere"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn nothing_is_held_at_start() {
    let (status, body) = call(Method::GET, "/api/held", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Array(Vec::new()));
}

#[tokio::test]
async fn a_recording_needs_points_and_a_sane_interval() {
    let (status, body) = call(Method::GET, "/api/recording", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["running"], false);
    assert_eq!(body["points"], Value::Array(Vec::new()));

    let (status, _) = call(Method::POST, "/api/recording/start", r#"{"interval_s":0}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = call(Method::POST, "/api/recording/start", r#"{"interval_s":10}"#).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "add a point to the recording first");
}

#[tokio::test]
async fn a_bad_subnet_is_refused_without_starting_a_scan() {
    let state = Arc::new(AppState::default());
    let request = |method: Method, path: &str, body: &str| {
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let refused = server::router(Arc::clone(&state))
        .oneshot(request(
            Method::POST,
            "/api/scan",
            r#"{"subnet":"192.168.10.0"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    let scan = server::router(state)
        .oneshot(request(Method::GET, "/api/scan", ""))
        .await
        .unwrap();
    let bytes = to_bytes(scan.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["state"], "idle");
}

/// A page elsewhere must not reach the plant through bacnetio, neither directly (its `Origin`)
/// nor by DNS rebinding (its name as `Host`).
#[tokio::test]
async fn foreign_pages_are_refused() {
    let mut state = AppState::default();
    state.web_address = Some(std::net::SocketAddr::from(([127, 0, 0, 1], 18099)));
    let app = server::router(Arc::new(state));
    let write = |host: &'static str, origin: Option<&'static str>| {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/devices/1/objects/1/1/write")
            .header("host", host)
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        request
            .body(Body::from(r#"{"value":"warm","priority":8}"#))
            .unwrap()
    };
    let status = |request: Request<Body>| {
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap().status() }
    };
    let rebinding = write("evil.example:18099", Some("http://evil.example:18099"));
    assert_eq!(status(rebinding).await, StatusCode::FORBIDDEN);
    let foreign_origin = write("127.0.0.1:18099", Some("http://evil.example"));
    assert_eq!(status(foreign_origin).await, StatusCode::FORBIDDEN);
    // our own page gets through the guard and on to the check of the value
    let own = write("127.0.0.1:18099", Some("http://127.0.0.1:18099"));
    assert_eq!(status(own).await, StatusCode::BAD_REQUEST);
}

#[cfg(not(target_os = "android"))]
#[tokio::test]
async fn serial_ports_are_listed() {
    let (status, body) = call(Method::GET, "/api/serial-ports", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}

#[cfg(not(target_os = "android"))]
#[tokio::test]
async fn listening_opens_only_a_listed_port_at_an_mstp_speed() {
    // a file is no serial port: the name would otherwise be opened as one
    let (status, body) = call(
        Method::POST,
        "/api/mstp/listen",
        r#"{"port":"/etc/passwd"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("not a serial port"),
        "{body}"
    );
}

//! The local web server: the user interface from `web/` and the JSON API it talks to
//! (docs/api.md). Listens on 127.0.0.1 only, because the API can write to the plant.

mod capturing;
mod commands;
mod devices;
mod exports;
mod files;
mod guard;
mod json;
mod logs;
mod plugins;
mod reading;
mod recording;
mod references;
mod remembered;
mod running;
#[cfg(feature = "sc")]
mod sc;
mod scanning;
#[cfg(not(target_os = "android"))]
mod serial;
mod session;
mod site;
mod workbooks;
mod writing;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde_json::json;
use tokio::net::TcpListener;

pub use session::{AppState, Shared};

pub const DEFAULT_PORT: u16 = 8081;

/// A site file brought back in: large sites have tens of thousands of points.
const MOST_SITE_BYTES: usize = 64 * 1024 * 1024;

#[derive(RustEmbed)]
#[folder = "web/"]
struct Assets;

/// MS/TP over a serial port: Windows and Linux; Android has none (docs/concept-mstp.md).
#[cfg(not(target_os = "android"))]
fn serial_routes() -> Router<Shared> {
    Router::new()
        .route("/api/serial-ports", get(serial::serial_ports))
        .route("/api/mstp/listen", post(serial::listen))
        .route("/api/mstp/search", post(serial::search))
        .route("/api/mstp/leave", post(serial::leave))
        .route("/api/mstp/trunk", get(serial::trunk))
}

#[cfg(target_os = "android")]
fn serial_routes() -> Router<Shared> {
    Router::new()
}

/// BACnet/SC, in every build with the feature `sc` (docs/concept-sc.md).
#[cfg(feature = "sc")]
fn sc_routes() -> Router<Shared> {
    Router::new()
        .route("/api/sc", get(sc::sc))
        .route("/api/sc/file", post(sc::file))
        .route("/api/sc/password", post(sc::password))
        .route("/api/sc/search", post(sc::search))
        .route("/api/sc/leave", post(sc::leave))
        .route("/api/sc/forget-key", post(sc::forget_key))
}

#[cfg(not(feature = "sc"))]
fn sc_routes() -> Router<Shared> {
    Router::new()
}

pub fn router(state: Shared) -> Router {
    let object = "/api/devices/{device}/objects/{type}/{instance}";
    Router::new()
        .route("/api/info", get(info))
        .route("/api/update", get(update))
        .route("/api/adapters", get(scanning::adapters))
        .merge(serial_routes())
        .merge(sc_routes())
        .route("/api/scan/stop", post(scanning::stop))
        .route(
            "/api/scan",
            get(scanning::scan_state).post(scanning::start_scan),
        )
        .route("/api/devices.csv", get(exports::devices_csv))
        .route("/api/devices.xlsx", get(workbooks::devices))
        .route("/api/site.xlsx", get(workbooks::site))
        .route("/api/log.xlsx", get(workbooks::log))
        .route("/api/recording.xlsx", get(workbooks::recording))
        .route("/api/devices/locate", post(scanning::locate))
        .route("/api/devices/{device}/points", get(devices::points))
        .route(
            "/api/devices/{device}/points/last",
            get(devices::last_points),
        )
        .route("/api/devices/{device}/read/stop", post(reading::stop))
        .route(
            "/api/devices/{device}/read",
            get(reading::progress).post(reading::start),
        )
        .route("/api/devices/{device}/points.csv", get(exports::points_csv))
        .route("/api/devices/{device}/points.xlsx", get(workbooks::points))
        .route(
            "/api/devices/{device}/points-priorities.xlsx",
            get(workbooks::points_priorities),
        )
        .route(
            "/api/devices/{device}/points-priorities.csv",
            get(exports::points_priorities_csv),
        )
        .route(
            "/api/devices/{device}/points.ede-states.csv",
            get(exports::points_ede_states),
        )
        .route("/api/devices/{device}/priorities", get(devices::priorities))
        .route("/api/devices/{device}/values", post(devices::live_values))
        .route("/api/devices/{device}/structure", get(devices::structure))
        .route("/api/devices/{device}/bbmd", get(devices::bbmd_tables))
        .route("/api/devices/{device}/restart", post(commands::restart))
        .route(
            "/api/devices/{device}/communication",
            post(commands::communication),
        )
        .route(
            "/api/devices/{device}/alarms/acknowledge",
            post(commands::acknowledge),
        )
        .route(
            &format!("{object}/schedule"),
            get(commands::schedule).post(commands::write_day),
        )
        .route(&format!("{object}/calendar"), get(commands::calendar))
        .route("/api/who-has", post(commands::who_has))
        .route("/api/events", get(commands::events))
        .route("/api/site/points", get(site::points))
        .route("/api/sites", get(site::list))
        .route("/api/sites/open", post(site::open))
        .route("/api/sites/delete", post(site::delete))
        .route("/api/sites/rename", post(site::rename))
        .route("/api/sites/grouping", post(site::grouping))
        .route(
            "/api/site.json",
            get(site::export)
                .post(site::import)
                .layer(DefaultBodyLimit::max(MOST_SITE_BYTES)),
        )
        .route(
            "/api/plugins",
            get(plugins::list)
                .post(plugins::install)
                .layer(DefaultBodyLimit::max(crate::plugin::MOST_PLUGIN_BYTES)),
        )
        .route("/api/plugins/{id}/switch", post(plugins::switch))
        .route("/api/plugins/{id}/remove", post(plugins::remove))
        .route("/api/plugins/{id}/page", get(plugins::page))
        .route(
            "/api/plugins/{id}/store",
            get(plugins::store)
                .post(plugins::save_store)
                .layer(DefaultBodyLimit::max(plugins::MOST_STORE_BYTES)),
        )
        .route("/api/site.ede.csv", get(exports::site_ede))
        .route("/api/site.ede-states.csv", get(exports::site_ede_states))
        .route(
            "/api/devices/{device}/points.ede.csv",
            get(exports::points_ede),
        )
        .route("/api/devices/{device}/alarms", get(devices::alarms))
        .route(
            "/api/devices/{device}/instance",
            post(writing::change_instance),
        )
        .route("/api/devices/{device}/clock", post(writing::set_clock))
        .route(
            "/api/devices/{device}/reference",
            get(references::compare).post(references::save),
        )
        .route(object, get(devices::object))
        .route(&format!("{object}/properties"), get(devices::properties))
        .route(&format!("{object}/value"), get(devices::value))
        .route(&format!("{object}/trend"), get(logs::trend))
        .route(&format!("{object}/trend.csv"), get(logs::trend_csv))
        .route(&format!("{object}/events"), get(logs::events))
        .route(&format!("{object}/events.csv"), get(logs::events_csv))
        .route(&format!("{object}/file"), get(files::file))
        .route(&format!("{object}/write"), post(writing::write_value))
        .route(&format!("{object}/property"), post(writing::write_property))
        .route(&format!("{object}/release"), post(writing::release))
        .route("/api/held", get(writing::held))
        .route("/api/log", get(writing::log))
        .route("/api/log.csv", get(writing::log_csv))
        .route("/api/recording", get(recording::recording))
        .route("/api/recording/points", post(recording::add_point))
        .route(
            "/api/recording/points/remove",
            post(recording::remove_point),
        )
        .route("/api/recording/start", post(recording::start))
        .route("/api/recording/mode", post(recording::set_mode))
        .route("/api/recording/stop", post(recording::stop))
        .route("/api/recording/clear", post(recording::clear))
        .route("/api/recording.csv", get(recording::csv))
        .route("/api/capture", get(capturing::state))
        .route("/api/capture/start", post(capturing::start))
        .route("/api/capture/stop", post(capturing::stop))
        .route("/api/capture/clear", post(capturing::clear))
        .route("/api/capture.pcapng", get(capturing::file))
        .fallback(static_file)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            guard::same_origin_only,
        ))
        .with_state(state)
}

/// Serves until the process ends. Takes the next free port if `port` is taken. The write log is
/// kept in `data_dir`; without one, it lasts only as long as the process. Started to open the
/// browser while a bacnetio already serves `data_dir`, it opens that one and ends (running.rs).
pub async fn serve(
    port: u16,
    open_in_browser: bool,
    data_dir: Option<PathBuf>,
    loopback: bool,
) -> anyhow::Result<()> {
    if open_in_browser
        && let Some(dir) = &data_dir
        && let Some(running) = running::already_running(dir).await
    {
        let url = local_url(running);
        println!("bacnetio is already running at {url}; opening it there.");
        open_browser(&url);
        return Ok(());
    }
    let mut state = match &data_dir {
        Some(dir) => AppState::with_data_dir(dir.clone())?,
        None => AppState::default(),
    };
    state.loopback = loopback;
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
        Ok(listener) => listener,
        Err(_) => TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .context("cannot open a local web server port")?,
    };
    let address = listener.local_addr()?;
    if let Some(dir) = &data_dir {
        running::remember(dir, address)
            .with_context(|| format!("cannot write to {}", dir.display()))?;
    }
    state.web_address = Some(address);
    let url = local_url(address);
    println!("bacnetio is running at {url}");
    println!("Close this window to quit.");
    if open_in_browser {
        open_browser(&url);
    }
    axum::serve(listener, router(Arc::new(state)))
        .await
        .context("the web server stopped")
}

fn local_url(address: SocketAddr) -> String {
    format!("http://127.0.0.1:{}/", address.port())
}

fn open_browser(url: &str) {
    let started = if cfg!(windows) {
        // "start" is a cmd built-in; the empty title keeps the URL from being taken as title
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        std::process::Command::new("xdg-open").arg(url).spawn()
    } else {
        return;
    };
    if started.is_err() {
        println!("Open {url} in your browser.");
    }
}

/// The version, for the About line.
async fn info() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        // MS/TP over a serial port (docs/concept-mstp.md): not on Android
        "mstp": cfg!(not(target_os = "android")),
        // BACnet/SC: left out of a build without the feature `sc`
        "sc": cfg!(feature = "sc"),
    }))
}

/// The newest release on GitHub and whether it is newer than this one; asked only by the page,
/// and only while the user lets it (docs/concept-update-check.md).
async fn update() -> Result<Json<serde_json::Value>, ApiError> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = crate::update::latest_release()
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    let newer = latest
        .as_ref()
        .is_some_and(|release| crate::update::is_newer(&release.version, current));
    Ok(Json(json!({
        "current": current,
        "latest": latest.map(|release| json!({ "version": release.version, "url": release.url })),
        "newer": newer,
    })))
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None if path == "index.html" => (
            StatusCode::OK,
            "bacnetio is running. The user interface is not built in yet; the API is under /api/.",
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

/// An error the API answers with: `{"error": "..."}` in plain words.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl ToString) -> Self {
        ApiError {
            status,
            message: message.to_string(),
        }
    }

    /// The request itself is wrong.
    pub fn bad_request(message: impl ToString) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// Not possible right now, e.g. before the first scan.
    pub fn conflict(message: impl ToString) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    /// Nothing of that name is there.
    pub fn not_found(message: impl ToString) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// The network or a device did not do what was asked.
    pub fn device(message: impl ToString) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, message)
    }

    pub fn internal(message: impl ToString) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

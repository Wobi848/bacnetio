//! BACnet/SC over HTTP (docs/concept-sc.md): the site's certificate files chosen and checked in
//! words, the connection to its hub with a search, and leaving it.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use bacnet_transport::sc_tls::TlsWebSocket;
use bacnet_types::enums::ObjectType;
use bacnet_types::primitives::ObjectIdentifier;
use serde::{Deserialize, Serialize};

use super::ApiError;
use super::scanning::{ScanView, begin_scan, spawn_scan};
use super::session::{AppState, ScanStatus, Shared};
use super::writing::record_command;
use crate::sc_certificates::{Line, View};
use crate::sc_identity::Identity;
use crate::sc_store;
use crate::scan::{self, WILDCARD_INSTANCE};

/// Answers to Who-Is come through the hub, from every device on the network.
const SC_WHO_IS_WAIT: Duration = Duration::from_secs(3);
/// The name of a chosen file as the line shows it, at most this long.
const MOST_NAME_CHARS: usize = 120;

#[derive(Serialize)]
pub struct ScView {
    certificates: View,
    /// The hub bacnetio is connected to, if it is.
    hub: Option<String>,
    /// The hub of the last connection, kept in the data folder, to offer again.
    last_hub: Option<String>,
    /// A private key is kept in the data folder.
    key_kept: bool,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

async fn view(state: &AppState) -> ScView {
    let certificates = state.sc.lock().expect("sc lock").view(now());
    let last_hub = state
        .data_dir
        .as_ref()
        .and_then(|dir| sc_store::kept_hub(dir).ok().flatten());
    let key_kept = state.data_dir.as_deref().is_some_and(sc_store::key_kept);
    ScView {
        certificates,
        hub: state.sc_hub().await,
        last_hub,
        key_kept,
    }
}

pub async fn sc(State(state): State<Shared>) -> Json<ScView> {
    Json(view(&state).await)
}

#[derive(Deserialize)]
pub struct FileQuery {
    line: Line,
    /// The file's name, as the browser gives it.
    name: String,
}

/// Only the name of the file, without a folder some browsers put before it.
fn file_name(name: &str) -> String {
    let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    let name: String = name.chars().take(MOST_NAME_CHARS).collect();
    if name.is_empty() {
        "file".to_string()
    } else {
        name
    }
}

/// A file chosen for a line, its bytes as the body; the lines it fills in words.
pub async fn file(
    State(state): State<Shared>,
    Query(query): Query<FileQuery>,
    body: Bytes,
) -> Json<ScView> {
    state.sc.lock().expect("sc lock").choose(
        query.line,
        &file_name(&query.name),
        body.to_vec(),
        None,
    );
    Json(view(&state).await)
}

#[derive(Deserialize)]
pub struct PasswordRequest {
    line: Line,
    password: String,
}

/// Opens the file waiting on a line with its password; the password is not kept.
pub async fn password(
    State(state): State<Shared>,
    Json(request): Json<PasswordRequest>,
) -> Result<Json<ScView>, ApiError> {
    let opened = state
        .sc
        .lock()
        .expect("sc lock")
        .open_with(request.line, &request.password);
    if !opened {
        return Err(ApiError::conflict(
            "no file waits for a password: choose it again",
        ));
    }
    Ok(Json(view(&state).await))
}

#[derive(Deserialize)]
pub struct SearchRequest {
    /// The hub's `wss://` URL.
    hub: String,
    /// Keep a key that came with a password; one without is kept anyway.
    #[serde(default)]
    remember_key: bool,
}

/// Connects to the hub with the certificates chosen and searches the SC network.
pub async fn search(
    State(state): State<Shared>,
    Json(request): Json<SearchRequest>,
) -> Result<StatusCode, ApiError> {
    let hub = request.hub.trim().to_string();
    if !hub.starts_with("wss://") || TlsWebSocket::validate_url(&hub).is_err() {
        return Err(ApiError::bad_request(format!(
            "{hub} is no hub address: wss://name:port/path, as the integrator gives it"
        )));
    }
    let tls = state
        .sc
        .lock()
        .expect("sc lock")
        .tls_config()
        .map_err(ApiError::conflict)?;
    let identity = match &state.data_dir {
        Some(dir) => Identity::kept_in(dir),
        None => Identity::new(),
    }
    .map_err(|error| ApiError::internal(format!("bacnetio's SC identity: {error}")))?;
    let joined = state.sc_hub().await.as_deref() == Some(hub.as_str());
    begin_scan(&state)?;
    let client = match state.join_sc(&hub, tls, identity).await {
        Ok(client) => client,
        Err(error) => {
            let reason = format!("{error:#}");
            state.set_scan(ScanStatus::Failed(reason.clone()));
            return Err(ApiError::device(reason));
        }
    };
    if !joined {
        log_hub(&state, &hub, "Connected to");
    }
    keep_files(&state, &hub, request.remember_key);
    // the site found is named after its hub, as one on IP after its network
    spawn_scan(state, hub_host(&hub).to_string(), |state| async move {
        let report = scan::run_in_range(
            &client,
            SC_WHO_IS_WAIT,
            Vec::new(),
            None,
            true,
            &|progress| state.set_scan(ScanStatus::Running(progress)),
        )
        .await?;
        let view = ScanView::new(&report, Vec::new());
        *state.devices.lock().expect("devices lock") = report.devices;
        Ok(view)
    });
    Ok(StatusCode::ACCEPTED)
}

/// `hub.example:443` of `wss://hub.example:443/bacnet`.
fn hub_host(hub: &str) -> &str {
    let rest = hub.strip_prefix("wss://").unwrap_or(hub);
    rest.split('/').next().unwrap_or(rest)
}

/// The files that made the connection, in the data folder for the next start; the key only when it
/// came without a password or the user asked for it.
fn keep_files(state: &AppState, hub: &str, remember_key: bool) {
    let Some(dir) = &state.data_dir else {
        return;
    };
    if let Err(error) = sc_store::keep_hub(dir, hub) {
        eprintln!(
            "Note: the SC hub could not be kept in {}: {error}",
            dir.display()
        );
    }
    let certificates = state.sc.lock().expect("sc lock");
    let keep_key = remember_key || !certificates.key_had_password();
    for line in [Line::Ca, Line::Own, Line::Key] {
        if line == Line::Key && !keep_key {
            continue;
        }
        if let Some((name, pem)) = certificates.pem(line)
            && let Err(error) = sc_store::keep(dir, line, &name, &pem)
        {
            eprintln!(
                "Note: the SC files could not be kept in {}: {error}",
                dir.display()
            );
        }
    }
}

/// Deletes the kept key and empties its line; the certificates stay. A connection open now keeps
/// running until it is left.
pub async fn forget_key(State(state): State<Shared>) -> Result<Json<ScView>, ApiError> {
    if let Some(dir) = &state.data_dir {
        sc_store::forget_key(dir)
            .map_err(|error| ApiError::internal(format!("cannot delete the SC key: {error}")))?;
    }
    state.sc.lock().expect("sc lock").forget_key();
    Ok(Json(view(&state).await))
}

/// Leaves the SC network; nothing to do when bacnetio is on none.
pub async fn leave(State(state): State<Shared>) -> Result<StatusCode, ApiError> {
    let left = state
        .leave_sc()
        .await
        .map_err(|error| ApiError::conflict(format!("{error:#}")))?;
    if let Some(hub) = left {
        log_hub(&state, &hub, "Left");
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The files kept at the last connection, back in the lines at start.
pub(super) fn restore(state: &AppState) {
    let Some(dir) = &state.data_dir else {
        return;
    };
    match sc_store::kept(dir) {
        Ok(kept) => {
            let mut certificates = state.sc.lock().expect("sc lock");
            for (line, name, pem) in kept {
                certificates.choose(line, &name, pem, None);
            }
        }
        Err(error) => eprintln!("Note: the kept SC files could not be read: {error}"),
    }
}

/// Joining an SC network makes bacnetio a node of it, so it is logged, under no device.
pub(super) fn log_hub(state: &Shared, hub: &str, done: &str) {
    let network = ObjectIdentifier::new(ObjectType::DEVICE, WILDCARD_INSTANCE)
        .expect("wildcard instance is valid");
    let text = format!("{done} the BACnet/SC hub at {hub}");
    record_command(state, WILDCARD_INSTANCE, network, text, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_named_without_its_folder() {
        assert_eq!(file_name("C:\\fakepath\\site.p12"), "site.p12");
        assert_eq!(file_name("/home/me/ca.pem"), "ca.pem");
        assert_eq!(file_name("  "), "file");
        assert_eq!(file_name(&"x".repeat(500)).len(), MOST_NAME_CHARS);
    }

    #[test]
    fn a_site_on_sc_is_named_after_its_hub() {
        assert_eq!(hub_host("wss://hub.example:443/bacnet"), "hub.example:443");
        assert_eq!(hub_host("wss://localhost:47443/"), "localhost:47443");
    }
}

//! MS/TP over HTTP (docs/concept-mstp.md): the serial ports to choose from, and listening to a
//! trunk, which sends nothing.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use bacnet_types::enums::ObjectType;
use bacnet_types::primitives::ObjectIdentifier;
use serde::Deserialize;

use super::ApiError;
use super::scanning::{ScanView, begin_scan, spawn_scan};
use super::session::{ScanStatus, Shared};
use super::writing::record_command;
use crate::connection::Trunk;
use crate::mstp::{self, Heard, SerialPortChoice};
use crate::scan::{self, WILDCARD_INSTANCE};

/// An MS/TP trunk answers Who-Is only as the token comes round: longer than on IP.
const MSTP_WHO_IS_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// A port the browser tests' simulator opened (a pseudo-terminal), listed beside the real ones
/// only with `--test-loopback`: the system lists no pseudo-terminals as serial ports.
const TEST_PORT_VARIABLE: &str = "BACNETIO_TEST_SERIAL";

/// What was being done and why it failed, without the layers in between, which repeat both.
fn in_words(error: &anyhow::Error) -> String {
    let cause = error.root_cause().to_string();
    let done = error.to_string();
    if done == cause {
        done
    } else {
        format!("{done}: {cause}")
    }
}

fn ports(loopback: bool) -> Result<Vec<SerialPortChoice>, ApiError> {
    let mut ports = mstp::serial_ports().map_err(|error| ApiError::internal(in_words(&error)))?;
    if loopback
        && let Ok(name) = std::env::var(TEST_PORT_VARIABLE)
        && !name.is_empty()
    {
        ports.insert(
            0,
            SerialPortChoice {
                name,
                description: Some("Simulator".to_string()),
                usb: true,
            },
        );
    }
    Ok(ports)
}

pub async fn serial_ports(
    State(state): State<Shared>,
) -> Result<Json<Vec<SerialPortChoice>>, ApiError> {
    Ok(Json(ports(state.loopback)?))
}

#[derive(Deserialize)]
pub struct ListenRequest {
    port: String,
    /// One speed to listen at; all of them, most common first, when left out.
    baud: Option<u32>,
}

/// Only a port the system lists: the name is opened as a file.
fn check_port(state: &Shared, port: &str) -> Result<(), ApiError> {
    if ports(state.loopback)?
        .iter()
        .any(|listed| listed.name == port)
    {
        return Ok(());
    }
    Err(ApiError::bad_request(format!(
        "{port} is not a serial port of this computer"
    )))
}

pub async fn listen(
    State(state): State<Shared>,
    Json(request): Json<ListenRequest>,
) -> Result<Json<Heard>, ApiError> {
    check_port(&state, &request.port)?;
    if state
        .trunk()
        .await
        .is_some_and(|trunk| trunk.port == request.port)
    {
        return Err(ApiError::conflict(
            "bacnetio is on this trunk: leave it before listening again",
        ));
    }
    let speeds = match request.baud {
        Some(baud) if mstp::SPEEDS.contains(&baud) => vec![baud],
        Some(baud) => {
            return Err(ApiError::bad_request(format!(
                "{baud} baud is no MS/TP speed: 9600, 19200, 38400, 76800 or 115200"
            )));
        }
        None => mstp::SPEEDS.to_vec(),
    };
    let port = request.port;
    let open = |baud| mstp::open_receive_only(&port, baud);
    let heard = mstp::listen(open, &speeds, mstp::LISTEN_PER_SPEED)
        .await
        .map_err(|error| ApiError::conflict(in_words(&error)))?;
    *state.heard.lock().expect("heard lock") = Some((port, heard.clone()));
    Ok(Json(heard))
}

#[derive(Deserialize)]
pub struct SearchRequest {
    port: String,
    baud: u32,
    station: u8,
}

/// Joins the trunk as `station` and searches it, once listening has heard it at that speed and
/// nobody at that address. Joining and leaving go into the log.
pub async fn search(
    State(state): State<Shared>,
    Json(request): Json<SearchRequest>,
) -> Result<StatusCode, ApiError> {
    check_port(&state, &request.port)?;
    let heard = state
        .heard
        .lock()
        .expect("heard lock")
        .clone()
        .filter(|(port, _)| *port == request.port)
        .map(|(_, heard)| heard)
        .ok_or_else(|| ApiError::conflict("listen to the trunk first"))?;
    mstp::may_join(&heard, request.baud, request.station).map_err(ApiError::conflict)?;
    let trunk = Trunk {
        port: request.port,
        baud: request.baud,
        station: request.station,
        max_master: mstp::MAX_MASTER,
    };
    begin_scan(&state)?;
    let joined = state.trunk().await.as_ref() == Some(&trunk);
    let client = match state.join(&trunk).await {
        Ok(client) => client,
        Err(error) => {
            let reason = in_words(&error);
            state.set_scan(ScanStatus::Failed(reason.clone()));
            return Err(ApiError::device(reason));
        }
    };
    if !joined {
        log_trunk(&state, &trunk, "Joined");
    }
    let scope = format!("{} · {}", trunk.port, trunk.baud);
    spawn_scan(state, scope, |state| async move {
        let report = scan::run_in_range(
            &client,
            MSTP_WHO_IS_WAIT,
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

/// The trunk bacnetio is on, or `null`.
pub async fn trunk(State(state): State<Shared>) -> Json<Option<Trunk>> {
    Json(state.trunk().await)
}

/// Leaves the trunk and closes its port; nothing to do when bacnetio is on none.
pub async fn leave(State(state): State<Shared>) -> Result<StatusCode, ApiError> {
    let left = state
        .leave()
        .await
        .map_err(|error| ApiError::conflict(in_words(&error)))?;
    if let Some(trunk) = left {
        log_trunk(&state, &trunk, "Left");
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Joining a trunk takes part in its timing, so it is logged like a command, under no device.
fn log_trunk(state: &Shared, trunk: &Trunk, done: &str) {
    let trunk_itself = ObjectIdentifier::new(ObjectType::DEVICE, WILDCARD_INSTANCE)
        .expect("wildcard instance is valid");
    let text = format!(
        "{done} the MS/TP trunk on {} at {} baud as station {}",
        trunk.port, trunk.baud, trunk.station
    );
    record_command(state, WILDCARD_INSTANCE, trunk_itself, text, None);
}

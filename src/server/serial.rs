//! MS/TP over HTTP (docs/concept-mstp.md): the serial ports to choose from, and listening to a
//! trunk, which sends nothing.

use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use super::ApiError;
use super::session::Shared;
use crate::mstp::{self, Heard, SerialPortChoice};

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

pub async fn listen(
    State(state): State<Shared>,
    Json(request): Json<ListenRequest>,
) -> Result<Json<Heard>, ApiError> {
    // only a port the system lists: the name is opened as a file
    if !ports(state.loopback)?
        .iter()
        .any(|port| port.name == request.port)
    {
        return Err(ApiError::bad_request(format!(
            "{} is not a serial port of this computer",
            request.port
        )));
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
    mstp::listen(open, &speeds, mstp::LISTEN_PER_SPEED)
        .await
        .map(Json)
        .map_err(|error| ApiError::conflict(in_words(&error)))
}

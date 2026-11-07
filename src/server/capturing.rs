//! Capturing bacnetio's own traffic over HTTP, and the file for Wireshark
//! (docs/concept-toolbox.md part 3).

use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;

use super::session::Shared;
use crate::capture::CaptureState;

pub async fn state(State(state): State<Shared>) -> Json<CaptureState> {
    Json(state.capture.state())
}

/// Starts capturing, from the connection open now and every one opened while it runs.
pub async fn start(State(state): State<Shared>) -> Json<CaptureState> {
    let client = state.open_client().await;
    state.capture.start(client.as_deref());
    Json(state.capture.state())
}

pub async fn stop(State(state): State<Shared>) -> Json<CaptureState> {
    state.capture.stop();
    Json(state.capture.state())
}

pub async fn clear(State(state): State<Shared>) -> Json<CaptureState> {
    state.capture.clear();
    Json(state.capture.state())
}

pub async fn file(State(state): State<Shared>) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/vnd.tcpdump.pcap"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"bacnetio.pcapng\"",
            ),
        ],
        state.capture.pcapng(),
    )
}

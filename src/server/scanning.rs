//! Finding devices over HTTP: the adapters, a scan with its progress, and one device by address.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::ApiError;
use super::json::{AdapterView, DeviceView, NoteView, phase_name};
use super::session::{AppState, NetworkRequest, ScanStatus, Shared};
use crate::connection::{self, Client};
use crate::diagnosis::{self, Note};
use crate::interfaces::{self, Adapter, Subnet};
use crate::present;
use crate::scan::{self, Phase, Progress, ScanReport};

pub async fn adapters(State(state): State<Shared>) -> Result<Json<Vec<AdapterView>>, ApiError> {
    let adapters =
        interfaces::list(state.loopback).map_err(|error| ApiError::internal(error.to_string()))?;
    let default = interfaces::pick_default(&adapters);
    Ok(Json(
        adapters
            .iter()
            .map(|adapter| AdapterView::new(adapter, Some(adapter) == default))
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct ScanRequest {
    #[serde(flatten)]
    network: NetworkRequest,
    /// Ask every address of the subnet; on unless false.
    sweep: Option<bool>,
    /// Ask every address of this subnet instead of the adapter's, e.g. "192.168.10.0/24" behind a
    /// VPN.
    subnet: Option<String>,
    /// Only devices with an ID from `id_from` to `id_to`.
    id_from: Option<u32>,
    id_to: Option<u32>,
}

/// What one scan asks, besides the connection.
struct ScanPlan {
    sweep: bool,
    port: u16,
    port_unavailable: Option<String>,
    instances: Option<(u32, u32)>,
    subnet: Option<Subnet>,
}

pub async fn start_scan(
    State(state): State<Shared>,
    Json(request): Json<ScanRequest>,
) -> Result<StatusCode, ApiError> {
    // all input is checked before the scan counts as running, so a bad request leaves no trace
    let (adapter, endpoint) = request.network.endpoint(state.loopback)?;
    let port = endpoint.port;
    let instances = match (request.id_from, request.id_to) {
        (None, None) => None,
        (from, to) => Some((from.unwrap_or(0), to.unwrap_or(scan::MAX_INSTANCE))),
    };
    let subnet = match request.subnet.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => Some(Subnet::parse(text).map_err(ApiError::bad_request)?),
    };
    {
        let mut status = state.scan.lock().expect("scan status lock");
        if matches!(*status, ScanStatus::Running(_)) {
            return Err(ApiError::conflict("a search is already running"));
        }
        *status = ScanStatus::Running(Progress::start());
    }
    let (client, port_unavailable) = match state.connect(&endpoint).await {
        Ok(opened) => opened,
        Err(error) => {
            let reason = format!("{error:#}");
            state.set_scan(ScanStatus::Failed(reason.clone()));
            return Err(ApiError::device(reason));
        }
    };
    let scope = request
        .subnet
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or_else(|| endpoint.bbmd.map(|bbmd| format!("BBMD {bbmd}")))
        .unwrap_or_else(|| adapter.ip.to_string());
    let plan = ScanPlan {
        subnet,
        sweep: request.sweep.unwrap_or(true),
        port,
        port_unavailable,
        instances,
    };
    tokio::spawn(async move {
        let status = tokio::select! {
            result = scan_with(&state, &client, &adapter, plan) => match result {
                Ok(view) => ScanStatus::Done(view),
                Err(error) => ScanStatus::Failed(format!("{error:#}")),
            },
            () = state.scan_stop.notified() => found_so_far(&state),
        };
        if matches!(status, ScanStatus::Done(_)) {
            let scan = serde_json::to_value(scan_view(&status)).expect("scan state is JSON");
            let title = state.kept_title(&scope);
            state.remember(|site| {
                site.title = title;
                site.name = scope;
                site.scan = Some(scan);
                site.points.clear();
            });
        }
        state.set_scan(status);
    });
    Ok(StatusCode::ACCEPTED)
}

/// A stopped search ends like a finished one, with the devices it has found.
fn found_so_far(state: &AppState) -> ScanStatus {
    let devices = match &*state.scan.lock().expect("scan status lock") {
        ScanStatus::Running(progress) => progress.devices.clone(),
        _ => Vec::new(),
    };
    let report = scan::ScanReport {
        devices,
        ..scan::ScanReport::default()
    };
    let view = ScanView::new(&report, Vec::new());
    *state.devices.lock().expect("devices lock") = report.devices;
    ScanStatus::Done(view)
}

/// Stops the running search; without one there is nothing to do.
pub async fn stop(State(state): State<Shared>) -> StatusCode {
    let running = matches!(
        *state.scan.lock().expect("scan status lock"),
        ScanStatus::Running(_)
    );
    if running {
        state.scan_stop.notify_one();
    }
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
pub struct LocateRequest {
    #[serde(flatten)]
    network: NetworkRequest,
    /// IP, or IP:PORT when not the BACnet port.
    address: String,
    /// Needed only when the device refuses to tell it, or sits behind a router.
    instance: Option<u32>,
    /// With `mac`: the device sits behind the router at `address`, on this network.
    #[serde(rename = "network")]
    remote_network: Option<u16>,
    mac: Option<String>,
}

/// Finds one device by its address, for when a scan does not reach it.
pub async fn locate(
    State(state): State<Shared>,
    Json(request): Json<LocateRequest>,
) -> Result<Json<DeviceView>, ApiError> {
    let address =
        connection::parse_address(request.address.trim()).map_err(ApiError::bad_request)?;
    if matches!(
        *state.scan.lock().expect("scan status lock"),
        ScanStatus::Running(_)
    ) {
        return Err(ApiError::conflict("a search is already running"));
    }
    let (_, endpoint) = request.network.endpoint(state.loopback)?;
    let (client, _) = state
        .connect(&endpoint)
        .await
        .map_err(|error| ApiError::device(format!("{error:#}")))?;
    let found = match (
        request.remote_network,
        request.mac.as_deref().map(str::trim),
    ) {
        (Some(network), Some(mac)) if !mac.is_empty() => {
            let instance = request.instance.ok_or_else(|| {
                ApiError::bad_request("a device behind a router needs its device ID")
            })?;
            let mac = scan::parse_station(mac).map_err(ApiError::bad_request)?;
            scan::find_routed(&client, instance, address, network, mac).await
        }
        _ => scan::find_one(&client, request.instance, address).await,
    };
    let device = found.map_err(|error| ApiError::device(format!("{error:#}")))?;
    let view = DeviceView::new(&device);
    let mut devices = state.devices.lock().expect("devices lock");
    devices.retain(|known| known.instance != device.instance);
    devices.push(device);
    Ok(Json(view))
}

async fn scan_with(
    state: &AppState,
    client: &Arc<Client>,
    adapter: &Adapter,
    plan: ScanPlan,
) -> anyhow::Result<ScanView> {
    let (sweep, targets) = diagnosis::plan_sweep(adapter, plan.subnet, plan.sweep, plan.port);
    let report = scan::run_in_range(
        client,
        scan::DEFAULT_WHO_IS_WAIT,
        targets,
        plan.instances,
        true,
        &|progress| state.set_scan(ScanStatus::Running(progress)),
    )
    .await?;
    let context = diagnosis::Context {
        sweep,
        link_local: adapter.ip.is_link_local(),
        port_unavailable: plan.port_unavailable,
    };
    let view = ScanView::new(&report, diagnosis::notes(&report, &context));
    *state.devices.lock().expect("devices lock") = report.devices;
    Ok(view)
}

#[derive(Serialize, Clone)]
pub struct ScanView {
    devices: Vec<DeviceView>,
    notes: Vec<NoteView<Note>>,
    routers: Vec<RouterView>,
    /// Addresses that speak BACnet but refused to tell their device ID.
    refused: Vec<Refusal>,
}

#[derive(Serialize, Clone)]
struct RouterView {
    address: String,
    networks: Vec<u16>,
}

#[derive(Serialize, Clone)]
struct Refusal {
    address: String,
    reason: String,
}

impl ScanView {
    fn new(report: &ScanReport, notes: Vec<Note>) -> Self {
        ScanView {
            devices: report.devices.iter().map(DeviceView::new).collect(),
            notes: notes.into_iter().map(NoteView::new).collect(),
            routers: report
                .routers
                .iter()
                .map(|router| RouterView {
                    address: present::address_text(router.address),
                    networks: router.networks.clone(),
                })
                .collect(),
            refused: report
                .refused
                .iter()
                .map(|(address, reason)| Refusal {
                    address: present::address_text(*address),
                    reason: reason.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct ScanState {
    state: &'static str,
    phase: Option<&'static str>,
    /// Addresses asked so far and in all, while sweeping.
    asked: usize,
    total: usize,
    found: usize,
    error: Option<String>,
    /// The devices found so far, while the scan runs; when it is done they are in `result`.
    #[serde(skip_serializing_if = "Option::is_none")]
    devices: Option<Vec<DeviceView>>,
    #[serde(flatten)]
    result: Option<ScanView>,
}

/// The scan as the interface follows it; before the first search the remembered site, marked
/// with `remembered_ms`.
pub async fn scan_state(State(state): State<Shared>) -> Json<serde_json::Value> {
    let status = state.scan.lock().expect("scan status lock");
    if matches!(*status, ScanStatus::Idle)
        && let Some(site) = &*state.site.lock().expect("site lock")
        && let Some(mut scan) = site.scan.clone()
    {
        scan["remembered_ms"] = site.saved_ms.into();
        return Json(scan);
    }
    Json(serde_json::to_value(scan_view(&status)).expect("scan state is JSON"))
}

fn scan_view(status: &ScanStatus) -> ScanState {
    let mut view = ScanState {
        state: "idle",
        phase: None,
        asked: 0,
        total: 0,
        found: 0,
        error: None,
        devices: None,
        result: None,
    };
    match status {
        ScanStatus::Idle => {}
        ScanStatus::Running(progress) => {
            view.state = "running";
            view.phase = Some(phase_name(progress.phase));
            view.found = progress.found;
            view.devices = Some(progress.devices.iter().map(DeviceView::new).collect());
            if let Phase::Sweep { asked, total } = progress.phase {
                (view.asked, view.total) = (asked, total);
            }
        }
        ScanStatus::Done(result) => {
            view.state = "done";
            view.found = result.devices.len();
            view.result = Some(result.clone());
        }
        ScanStatus::Failed(reason) => {
            view.state = "failed";
            view.error = Some(reason.clone());
        }
    }
    view
}

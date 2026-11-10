//! The recording: which points, sampling in the background, and the data for chart and CSV.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use axum::Json;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use super::ApiError;
use super::devices::object_id;
use super::json::ObjectRef;
use super::session::{AppState, Shared};
use crate::export;
use crate::points::{self, Point};
use crate::present;
use crate::record::{self, MIN_INTERVAL, Mode, RecordedPoint};
use crate::watch;

const BACKUP_EVERY: Duration = Duration::from_secs(60);
const BACKUP_FILE: &str = "recording-utc.csv";

/// The background task that samples, and its interval.
pub struct Sampler {
    task: JoinHandle<()>,
    interval: Duration,
}

#[derive(Serialize)]
pub struct RecordingView {
    running: bool,
    interval_s: Option<u64>,
    points: Vec<SeriesView>,
}

#[derive(Serialize)]
struct SeriesView {
    device: u32,
    #[serde(flatten)]
    object: ObjectRef,
    name: Option<String>,
    units: Option<String>,
    /// `[milliseconds since 1970, value or null]`, oldest first.
    samples: Vec<(u64, Option<f64>)>,
    mode: Mode,
    cov_refused: bool,
}

pub async fn recording(State(state): State<Shared>) -> Json<RecordingView> {
    let interval = crate::locked(&state.sampler)
        .as_ref()
        .map(|sampler| sampler.interval);
    let recording = crate::locked(&state.recording);
    let points = recording
        .series()
        .iter()
        .map(|series| SeriesView {
            device: series.point.device,
            object: ObjectRef::new(series.point.object),
            name: series.point.name.clone(),
            units: series.point.units.map(present::unit_text),
            samples: series
                .samples
                .iter()
                .map(|sample| (crate::unix_time::millis(sample.time), sample.value))
                .collect(),
            mode: series.mode,
            cov_refused: series.cov_refused,
        })
        .collect();
    Json(RecordingView {
        running: interval.is_some(),
        interval_s: interval.map(|interval| interval.as_secs()),
        points,
    })
}

#[derive(Deserialize)]
pub struct PointRequest {
    device: u32,
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
}

/// Adds a point; its name and unit are read from the device once.
pub async fn add_point(
    State(state): State<Shared>,
    Json(request): Json<PointRequest>,
) -> Result<Json<RecordingView>, ApiError> {
    let object = object_id(request.object_type, request.instance)?;
    let client = state.client_for(request.device, None).await?;
    let mut point = Point::new(object);
    points::read_single(&client, request.device, &mut point, &|_| {}).await;
    crate::locked(&state.recording)
        .add(RecordedPoint {
            device: request.device,
            object,
            name: point.name,
            units: point.units,
        })
        .map_err(ApiError::conflict)?;
    Ok(recording(State(state)).await)
}

pub async fn remove_point(
    State(state): State<Shared>,
    Json(request): Json<PointRequest>,
) -> Result<Json<RecordingView>, ApiError> {
    let object = object_id(request.object_type, request.instance)?;
    crate::locked(&state.recording).remove(request.device, object);
    Ok(recording(State(state)).await)
}

#[derive(Deserialize)]
pub struct ModeRequest {
    device: u32,
    #[serde(rename = "type")]
    object_type: u32,
    instance: u32,
    mode: Mode,
}

/// Poll a point at the interval, or record every report its device sends.
pub async fn set_mode(
    State(state): State<Shared>,
    Json(request): Json<ModeRequest>,
) -> Result<Json<RecordingView>, ApiError> {
    let object = object_id(request.object_type, request.instance)?;
    let found = crate::locked(&state.recording).set_mode(request.device, object, request.mode);
    if !found {
        return Err(ApiError::bad_request("this point is not in the recording"));
    }
    Ok(recording(State(state)).await)
}

#[derive(Deserialize)]
pub struct StartRequest {
    interval_s: u64,
}

/// Starts sampling, or changes the interval of a running recording.
pub async fn start(
    State(state): State<Shared>,
    Json(request): Json<StartRequest>,
) -> Result<Json<RecordingView>, ApiError> {
    let interval = Duration::from_secs(request.interval_s);
    if interval < MIN_INTERVAL {
        return Err(ApiError::bad_request(format!(
            "the interval must be at least {} s",
            MIN_INTERVAL.as_secs()
        )));
    }
    if crate::locked(&state.recording).series().is_empty() {
        return Err(ApiError::conflict("add a point to the recording first"));
    }
    state.client().await?;
    let task = tokio::spawn(sample_forever(Arc::clone(&state), interval));
    let old = crate::locked(&state.sampler).replace(Sampler { task, interval });
    if let Some(old) = old {
        old.task.abort();
    }
    Ok(recording(State(state)).await)
}

pub async fn stop(State(state): State<Shared>) -> Json<RecordingView> {
    if let Some(sampler) = crate::locked(&state.sampler).take() {
        sampler.task.abort();
    }
    recording(State(state)).await
}

/// Drops the samples, keeps the points.
pub async fn clear(State(state): State<Shared>) -> Json<RecordingView> {
    crate::locked(&state.recording).clear_samples();
    recording(State(state)).await
}

#[derive(Deserialize)]
pub struct CsvQuery {
    /// The viewer's offset from UTC in minutes, for local times.
    #[serde(default)]
    pub(super) offset_min: i32,
}

pub async fn csv(State(state): State<Shared>, Query(query): Query<CsvQuery>) -> impl IntoResponse {
    let recording = crate::locked(&state.recording);
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"recording.csv\"",
            ),
        ],
        export::recording_csv(&recording, query.offset_min),
    )
}

/// Takes the client anew each round and lets go of it between rounds: a search on another
/// adapter can close the old connection, and the recording goes on with the new one.
async fn sample_forever(state: Shared, interval: Duration) {
    let mut ticks = tokio::time::interval(interval);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut saved = Instant::now();
    let mut first = true;
    loop {
        ticks.tick().await;
        let Ok(client) = state.client().await else {
            continue;
        };
        let points = crate::locked(&state.recording).points_with_mode();
        let time = SystemTime::now();
        let mut round = Vec::with_capacity(points.len());
        for (point, mode) in points {
            if mode == Mode::Cov {
                let reporting =
                    watch::keep_subscribed(&state.watch, &client, point.device, point.object).await;
                if !reporting {
                    crate::locked(&state.recording).cov_refused(point.device, point.object);
                } else if !first {
                    // the reports come in by themselves (session.rs); only the start needs a value,
                    // since a device reports again only when the value changes
                    continue;
                }
            }
            let value = watch::read_now(&client, point.device, point.object)
                .await
                .ok()
                .and_then(|(live, _)| record::number(&live.value));
            round.push((point, value));
        }
        first = false;
        crate::locked(&state.recording).record(time, &round);
        if saved.elapsed() >= BACKUP_EVERY {
            save_backup(&state);
            saved = Instant::now();
        }
    }
}

/// Writes the recording to the data folder, so a crash or an empty battery loses at most a
/// minute. Times are UTC: the server does not know the viewer's time zone.
fn save_backup(state: &AppState) {
    let Some(dir) = &state.data_dir else {
        return;
    };
    let csv = export::recording_csv(&crate::locked(&state.recording), 0);
    let path = dir.join(BACKUP_FILE);
    let written = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, csv));
    if let Err(reason) = written {
        eprintln!(
            "Warning: cannot save the recording to {}: {reason}",
            path.display()
        );
    }
}

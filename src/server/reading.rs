//! Reading a device's points in the background: the interface starts it, then follows the object
//! list and the values as they come in, with every request in the request log.

use std::collections::VecDeque;
use std::time::Instant;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;

use super::ApiError;
use super::json::{NoteView, PointView};
use super::session::Shared;
use crate::observe::{Event, Request};
use crate::points::{self, Point, ReadNote};

/// Requests kept per device in the request log.
const LOGGED_REQUESTS: usize = 500;

#[derive(Default)]
pub struct ReadJob {
    running: bool,
    started: Option<Instant>,
    /// Object list entries known, and their number once read.
    listed: usize,
    total: Option<usize>,
    /// Points whose values are read.
    read: usize,
    points: Vec<Point>,
    notes: Vec<ReadNote>,
    error: Option<String>,
    requests: VecDeque<(Instant, Request)>,
    /// The running reading, for stopping it.
    task: Option<tokio::task::AbortHandle>,
}

impl ReadJob {
    fn take(&mut self, event: Event<'_>) {
        match event {
            Event::Request(request) => {
                if self.requests.len() == LOGGED_REQUESTS {
                    self.requests.pop_front();
                }
                self.requests.push_back((Instant::now(), request));
            }
            Event::Listed { read, total } => {
                self.listed = read;
                self.total = total;
            }
            Event::Values { read, points } => {
                self.read = read;
                self.listed = points.len();
                self.total = Some(points.len());
                self.points = points.to_vec();
            }
        }
    }
}

#[derive(Serialize)]
pub struct ReadView {
    /// "idle", "running", "done" or "failed".
    state: &'static str,
    /// "limits", "list" or "values" while running.
    phase: Option<&'static str>,
    listed: usize,
    total: Option<usize>,
    read: usize,
    points: Vec<PointView>,
    notes: Vec<NoteView<ReadNote>>,
    error: Option<String>,
    requests: Vec<RequestView>,
}

#[derive(Serialize)]
struct RequestView {
    /// Milliseconds since the reading started.
    at_ms: u64,
    what: String,
    took_ms: u64,
    failed: Option<String>,
}

impl ReadView {
    fn new(job: &ReadJob) -> Self {
        let state = match (job.running, &job.error, job.started) {
            (true, _, _) => "running",
            (false, Some(_), _) => "failed",
            (false, None, Some(_)) => "done",
            (false, None, None) => "idle",
        };
        let phase = job
            .running
            .then_some(match (job.total, job.points.is_empty()) {
                (None, _) => "limits",
                (Some(_), true) => "list",
                (Some(_), false) => "values",
            });
        let started = job.started.unwrap_or_else(Instant::now);
        ReadView {
            state,
            phase,
            listed: job.listed,
            total: job.total,
            read: job.read,
            points: job.points.iter().map(PointView::new).collect(),
            notes: job.notes.iter().cloned().map(NoteView::new).collect(),
            error: job.error.clone(),
            requests: job
                .requests
                .iter()
                .map(|(at, request)| RequestView {
                    at_ms: at.saturating_duration_since(started).as_millis() as u64,
                    what: request.what.clone(),
                    took_ms: request.took.as_millis() as u64,
                    failed: request.failed.clone(),
                })
                .collect(),
        }
    }
}

/// Starts reading the device in the background, unless that already runs.
pub async fn start(
    State(state): State<Shared>,
    Path(device): Path<u32>,
) -> Result<Json<ReadView>, ApiError> {
    let client = state.client().await?;
    {
        let mut jobs = crate::locked(&state.reads);
        let job = jobs.entry(device).or_default();
        if !job.running {
            *job = ReadJob {
                running: true,
                started: Some(Instant::now()),
                ..ReadJob::default()
            };
            let known_list = take_searched_list(&state, device);
            let state = std::sync::Arc::clone(&state);
            let task = tokio::spawn(async move {
                let observe = |event: Event<'_>| {
                    if let Some(job) = crate::locked(&state.reads).get_mut(&device) {
                        job.take(event);
                    }
                };
                let result = points::read_device_observed(
                    &client,
                    device,
                    known_list,
                    &observe,
                    &state.precedence,
                )
                .await;
                finish(&state, device, result);
            });
            job.task = Some(task.abort_handle());
        }
    }
    Ok(progress(State(state), Path(device)).await)
}

/// Stops the running reading of the device. What has been read stays, with a note about the rest.
pub async fn stop(State(state): State<Shared>, Path(device): Path<u32>) -> StatusCode {
    let reading = {
        let mut jobs = crate::locked(&state.reads);
        let Some(job) = jobs.get_mut(&device).filter(|job| job.running) else {
            return StatusCode::NO_CONTENT;
        };
        if let Some(task) = job.task.take() {
            task.abort();
        }
        if job.points.is_empty() {
            // nothing read yet: what was known of the device stays as it is
            job.running = false;
            return StatusCode::NO_CONTENT;
        }
        let unread = job.points.len().saturating_sub(job.read);
        let mut notes = std::mem::take(&mut job.notes);
        notes.push(ReadNote::StoppedByUser { unread });
        points::DeviceReading {
            points: job.points.clone(),
            notes,
        }
    };
    finish(&state, device, Ok(reading));
    StatusCode::NO_CONTENT
}

/// The object list the search read for this device, once: reading again asks the device itself.
fn take_searched_list(
    state: &Shared,
    device: u32,
) -> Option<(Vec<bacnet_types::primitives::ObjectIdentifier>, bool)> {
    let mut devices = crate::locked(&state.devices);
    let list = &mut devices
        .iter_mut()
        .find(|found| found.instance == device)?
        .objects;
    Some((list.objects.take()?, list.in_parts))
}

fn finish(state: &Shared, device: u32, result: anyhow::Result<points::DeviceReading>) {
    let mut jobs = crate::locked(&state.reads);
    let Some(job) = jobs.get_mut(&device) else {
        return;
    };
    job.running = false;
    match result {
        Ok(reading) => {
            job.read = reading.points.len();
            job.points = reading.points.clone();
            job.notes = reading.notes;
            let remembered = serde_json::json!({
                "points": reading.points.iter().map(PointView::new).collect::<Vec<_>>(),
                "notes": Vec::<()>::new(),
            });
            state.remember(|site| {
                site.points.insert(device.to_string(), remembered);
            });
            crate::locked(&state.readings).insert(device, reading.points);
        }
        Err(error) => job.error = Some(format!("{error:#}")),
    }
}

/// How far the reading of the device is, with the points so far and the request log.
pub async fn progress(State(state): State<Shared>, Path(device): Path<u32>) -> Json<ReadView> {
    let jobs = crate::locked(&state.reads);
    Json(ReadView::new(
        jobs.get(&device).unwrap_or(&ReadJob::default()),
    ))
}

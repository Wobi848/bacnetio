//! The open BACnet connection and the scan running on it. One connection at a time: a new scan
//! on another adapter, port or BBMD, or on an MS/TP trunk, replaces it.

use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::ApiError;
use super::json::ObjectRef;
use super::recording::Sampler;
use super::scanning::ScanView;
use crate::address::Address;
use crate::alarms;
use crate::connection::{self, BACNET_PORT, Client, Connection, Endpoint, Trunk};
use crate::interfaces::{self, Adapter};
use crate::points::Point;
use crate::record::Recording;
use crate::scan::{self, Device, Progress};
use crate::storage;
use crate::watch::Watch;
use crate::write::Journal;
use bacnet_client::client::{ReceivedCOVNotification, ReceivedEventNotification};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

#[derive(Default)]
pub struct AppState {
    connection: tokio::sync::Mutex<Option<Open>>,
    pub(super) scan: Mutex<ScanStatus>,
    /// Ends a running search, keeping the devices found so far.
    pub(super) scan_stop: tokio::sync::Notify,
    /// The last reading of each device, for the CSV export.
    pub readings: Mutex<HashMap<u32, Vec<Point>>>,
    /// Alarm messages received, the last [`KEPT_EVENTS`].
    pub(super) events: Mutex<VecDeque<EventView>>,
    /// The site remembered on disk, shown until a new search (site.rs).
    pub(super) site: Mutex<Option<super::site::Site>>,
    /// Readings running in the background or finished, with their request logs.
    pub reads: Mutex<HashMap<u32, super::reading::ReadJob>>,
    pub journal: Mutex<Journal>,
    /// Values the devices report by COV.
    pub watch: Mutex<Watch>,
    /// Where the write log and the recording backup are kept; none means memory only.
    pub data_dir: Option<PathBuf>,
    /// The devices of the last finished scan, for the CSV export.
    pub devices: Mutex<Vec<Device>>,
    pub recording: Mutex<Recording>,
    /// What a user waits for goes before the reading of a list (precedence.rs).
    pub precedence: crate::precedence::Precedence,
    pub sampler: Mutex<Option<Sampler>>,
    /// The loopback interface counts as an adapter (browser tests).
    pub loopback: bool,
    /// Where the web server listens; requests must name it (guard.rs). `None` in tests that call
    /// the router directly.
    pub web_address: Option<std::net::SocketAddr>,
    /// What listening to an MS/TP trunk last heard, and on which port: joining needs it
    /// (docs/concept-mstp.md).
    #[cfg(not(target_os = "android"))]
    pub(super) heard: Mutex<Option<(String, crate::mstp::Heard)>>,
    /// bacnetio's own traffic, while a capture runs (docs/concept-toolbox.md part 3).
    pub(super) capture: Arc<crate::capture::Capture>,
}

pub type Shared = Arc<AppState>;

struct Open {
    key: Key,
    connection: Connection,
}

/// What a connection was opened with: interface, port and BBMD, or an MS/TP trunk.
#[derive(PartialEq)]
enum Key {
    Ip(Ipv4Addr, u16, Option<SocketAddrV4>),
    Mstp(Trunk),
}

#[derive(Default)]
pub(super) enum ScanStatus {
    #[default]
    Idle,
    Running(Progress),
    Done(ScanView),
    Failed(String),
}

impl AppState {
    /// Every write and release is appended here, so held values survive a restart.
    pub fn journal_file(&self) -> Option<PathBuf> {
        self.data_dir
            .as_ref()
            .map(|dir| dir.join(storage::JOURNAL_FILE))
    }

    /// State with the write log of earlier sessions, read from `data_dir`.
    pub fn with_data_dir(data_dir: PathBuf) -> anyhow::Result<Self> {
        let journal_file = data_dir.join(storage::JOURNAL_FILE);
        let (entries, unreadable) = storage::load(&journal_file)
            .with_context(|| format!("cannot read {}", journal_file.display()))?;
        if unreadable > 0 {
            eprintln!(
                "Note: {unreadable} line(s) of {} could not be read and were skipped.",
                journal_file.display()
            );
        }
        let mut journal = Journal::default();
        entries.into_iter().for_each(|entry| journal.record(entry));
        let site = super::site::load(&data_dir).unwrap_or_else(|reason| {
            eprintln!("Note: the remembered site could not be read and starts empty: {reason}");
            None
        });
        let state = AppState {
            site: Mutex::new(site),
            journal: Mutex::new(journal),
            data_dir: Some(data_dir),
            ..AppState::default()
        };
        super::remembered::restore(&state);
        Ok(state)
    }

    /// The client of the open connection; there is none before the first scan.
    pub async fn client(&self) -> Result<Arc<Client>, ApiError> {
        match &*self.connection.lock().await {
            Some(open) => Ok(Arc::clone(&open.connection.client)),
            None => Err(ApiError::conflict("search for devices first")),
        }
    }

    /// The client, made ready for `device`: opens a connection on the default adapter if none is
    /// open, and finds the device at `address` or by Who-Is if the client does not know it. So
    /// values held from an earlier session can be released without a scan first.
    pub async fn client_for(
        self: &Arc<Self>,
        device: u32,
        address: Option<Address>,
    ) -> Result<Arc<Client>, ApiError> {
        let client = self.reachable_client(device, address).await?;
        if client.get_device(device).await.is_none() {
            scan::locate(&client, Some(device), address, scan::DEFAULT_WHO_IS_WAIT)
                .await
                .map_err(|error| ApiError::device(format!("{error:#}")))?;
        }
        Ok(client)
    }

    /// As `client_for`, for a device whose communication bacnetio stopped: it answers nothing but
    /// the command to talk again, so it is taken as being at `address` without asking.
    pub async fn client_for_silent(
        self: &Arc<Self>,
        device: u32,
        address: Option<Address>,
    ) -> Result<Arc<Client>, ApiError> {
        let Some(address) = address else {
            return self.client_for(device, None).await;
        };
        let client = self.reachable_client(device, Some(address)).await?;
        if client.get_device(device).await.is_none() {
            client
                .add_device(device, &address.mac())
                .await
                .map_err(|error| ApiError::device(format!("{error:#}")))?;
        }
        Ok(client)
    }

    /// The open client, or one on the default adapter, on the link the device was reached over.
    async fn reachable_client(
        self: &Arc<Self>,
        device: u32,
        address: Option<Address>,
    ) -> Result<Arc<Client>, ApiError> {
        // a value held over a trunk is released over that trunk: bacnetio joins one only after
        // listening, never by itself (docs/concept-mstp.md)
        let on_trunk = self.trunk().await.is_some();
        if let Some(reason) = other_link(device, address, on_trunk) {
            return Err(ApiError::conflict(reason));
        }
        match self.client().await {
            Ok(client) => Ok(client),
            Err(_) => {
                let (_, endpoint) = NetworkRequest::default().endpoint(self.loopback)?;
                Ok(self
                    .connect(&endpoint)
                    .await
                    .map_err(|error| ApiError::device(format!("{error:#}")))?
                    .0)
            }
        }
    }

    /// Reuses the open connection if it has the same endpoint, else replaces it.
    pub(super) async fn connect(
        self: &Arc<Self>,
        endpoint: &Endpoint,
    ) -> anyhow::Result<(Arc<Client>, Option<String>)> {
        let key = Key::Ip(endpoint.interface, endpoint.port, endpoint.bbmd);
        self.connect_with(key, || connection::open(endpoint)).await
    }

    /// Joins the MS/TP trunk, or stays on it when already there with the same settings.
    #[cfg(not(target_os = "android"))]
    pub(super) async fn join(self: &Arc<Self>, trunk: &Trunk) -> anyhow::Result<Arc<Client>> {
        let key = Key::Mstp(trunk.clone());
        let (client, _) = self
            .connect_with(key, || connection::open_mstp(trunk))
            .await?;
        Ok(client)
    }

    /// Leaves the MS/TP trunk, closing its port; the trunk it was on, if bacnetio was on one.
    pub(super) async fn leave(&self) -> anyhow::Result<Option<Trunk>> {
        let mut current = self.connection.lock().await;
        let Some(Open {
            key: Key::Mstp(trunk),
            ..
        }) = current.as_ref()
        else {
            return Ok(None);
        };
        let trunk = trunk.clone();
        if let Some(open) = current.take() {
            open.connection.close_when_idle(REQUESTS_END_WAIT).await?;
        }
        Ok(Some(trunk))
    }

    /// The trunk bacnetio is on, if it is on one.
    pub(super) async fn trunk(&self) -> Option<Trunk> {
        match &*self.connection.lock().await {
            Some(Open {
                key: Key::Mstp(trunk),
                ..
            }) => Some(trunk.clone()),
            _ => None,
        }
    }

    async fn connect_with<F>(
        self: &Arc<Self>,
        key: Key,
        open: impl FnOnce() -> F,
    ) -> anyhow::Result<(Arc<Client>, Option<String>)>
    where
        F: std::future::Future<Output = anyhow::Result<Connection>>,
    {
        let mut current = self.connection.lock().await;
        if let Some(open) = current.as_ref().filter(|open| open.key == key) {
            let connection = &open.connection;
            return Ok((
                Arc::clone(&connection.client),
                connection.port_unavailable.clone(),
            ));
        }
        if let Some(old) = current.take() {
            old.connection.close_when_idle(REQUESTS_END_WAIT).await?;
            // a search over IP leaves the trunk as the button does, and the log says so
            #[cfg(not(target_os = "android"))]
            if let Key::Mstp(trunk) = &old.key {
                super::serial::log_trunk(self, trunk, "Left");
            }
        }
        let connection = open().await?;
        self.capture.follow(&connection.client);
        tokio::spawn(follow_notifications(
            Arc::clone(self),
            connection.client.cov_notifications(),
        ));
        tokio::spawn(follow_events(
            Arc::clone(self),
            connection.client.event_notifications(),
        ));
        tokio::spawn(keep_devices_known(
            Arc::clone(self),
            Arc::downgrade(&connection.client),
            matches!(key, Key::Mstp(_)),
        ));
        let found = (
            Arc::clone(&connection.client),
            connection.port_unavailable.clone(),
        );
        *current = Some(Open { key, connection });
        Ok(found)
    }

    /// The client of the connection open now, if there is one.
    pub(super) async fn open_client(&self) -> Option<Arc<Client>> {
        self.connection
            .lock()
            .await
            .as_ref()
            .map(|open| Arc::clone(&open.connection.client))
    }

    pub(super) fn set_scan(&self, status: ScanStatus) {
        *self.scan.lock().expect("scan status lock") = status;
    }
}

/// How long closing a connection waits for the requests under way; new ones wait for the lock.
const REQUESTS_END_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Alarm messages kept, the latest ones: a chattering device must not fill the memory.
const KEPT_EVENTS: usize = 500;
const KEEP_KNOWN_EVERY: std::time::Duration = std::time::Duration::from_secs(60);
/// Well before the client's 10 minutes, so a lost answer still leaves a second try.
const ASK_AGAIN_AFTER: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// One alarm message from a device: it reports an object changing state, unasked.
#[derive(Clone, Serialize)]
pub struct EventView {
    /// Seconds since 1970-01-01 UTC, when bacnetio received it.
    time: u64,
    device: u32,
    #[serde(flatten)]
    object: ObjectRef,
    from: String,
    to: String,
    text: Option<String>,
}

/// Collects the alarm messages until the connection closes. Devices send them to computers listed in
/// their notification classes, so this may stay empty.
async fn follow_events(state: Shared, mut events: broadcast::Receiver<ReceivedEventNotification>) {
    loop {
        match events.recv().await {
            Ok(received) => {
                let event = received.notification;
                let view = EventView {
                    time: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |since| since.as_secs()),
                    device: event.initiating_device_identifier.instance_number(),
                    object: ObjectRef::new(event.event_object_identifier),
                    from: alarms::state_text(event.from_state),
                    to: alarms::state_text(event.to_state),
                    text: event.message_text.clone(),
                };
                let mut kept = state.events.lock().expect("events lock");
                if kept.len() == KEPT_EVENTS {
                    kept.pop_front();
                }
                kept.push_back(view);
            }
            // missed some under load; the device's alarm list still has them
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Keeps the live values up to date until the connection closes.
async fn follow_notifications(
    state: Shared,
    mut notifications: broadcast::Receiver<ReceivedCOVNotification>,
) {
    loop {
        match notifications.recv().await {
            Ok(received) => {
                let time = SystemTime::now();
                let notification = &received.notification;
                let device = notification.initiating_device_identifier.instance_number();
                let object = notification.monitored_object_identifier;
                let value = {
                    let mut watch = state.watch.lock().expect("watch lock");
                    watch.apply(notification, time);
                    watch
                        .reported(device, object)
                        .and_then(|live| crate::record::number(&live.value))
                };
                if state.sampler.lock().expect("sampler lock").is_some() {
                    state
                        .recording
                        .lock()
                        .expect("recording lock")
                        .record_report(device, object, time, value);
                }
            }
            // missed some; the next notification brings the current value again
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// The client forgets a device 10 minutes after its last I-Am, answers to requests do not count,
/// and then every request to it fails. Before that, ask each known device again, or for one that
/// only answers at its address, note it again; a device that no longer answers is forgotten.
/// Holds the client only weakly between rounds, so a search on another adapter can close it.
/// Only devices of the client's own link: a device found over IP is not on the trunk, and its
/// address means nothing there.
async fn keep_devices_known(state: Shared, client: Weak<Client>, on_trunk: bool) {
    let mut ticks = tokio::time::interval(KEEP_KNOWN_EVERY);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        let current = state
            .connection
            .lock()
            .await
            .as_ref()
            .map(|open| Arc::clone(&open.connection.client));
        let Some(client) = current.filter(|current| Arc::as_ptr(current) == client.as_ptr()) else {
            return;
        };
        let known: Vec<(u32, Option<Address>)> = state
            .devices
            .lock()
            .expect("devices lock")
            .iter()
            .filter(|device| matches!(device.address, Address::Mstp(_)) == on_trunk)
            .map(|device| {
                let by_address = !device.answered_who_is && device.remote.is_none();
                (device.instance, by_address.then_some(device.address))
            })
            .collect();
        for (instance, by_address) in known {
            let fresh = client
                .get_device(instance)
                .await
                .is_some_and(|found| found.last_seen.elapsed() < ASK_AGAIN_AFTER);
            if fresh {
                continue;
            }
            match by_address {
                Some(address) => {
                    let _ = client.add_device(instance, &address.mac()).await;
                }
                None => scan::ask_again(&client, instance).await,
            }
        }
    }
}

/// Why `device`, last reached at `address`, cannot be reached on the connection bacnetio has now:
/// it sits on an MS/TP trunk and bacnetio is not on one, or the other way round.
fn other_link(device: u32, address: Option<Address>, on_trunk: bool) -> Option<String> {
    match (address, on_trunk) {
        (Some(Address::Mstp(station)), false) => Some(format!(
            "device {device} sits on an MS/TP trunk as station {station}: listen to the trunk \
             and join it first"
        )),
        (Some(Address::Ip(_)), true) => Some(format!(
            "device {device} is on BACnet/IP: leave the trunk and search on BACnet/IP first"
        )),
        _ => None,
    }
}

/// Adapter, port and BBMD of a request; all optional.
#[derive(Deserialize, Default)]
pub struct NetworkRequest {
    /// IP of the adapter; the default one if missing.
    interface: Option<Ipv4Addr>,
    port: Option<u16>,
    /// IP or IP:PORT.
    bbmd: Option<String>,
}

impl NetworkRequest {
    pub(super) fn endpoint(&self, loopback: bool) -> Result<(Adapter, Endpoint), ApiError> {
        let bbmd = match self.bbmd.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(text) => Some(connection::parse_address(text).map_err(ApiError::bad_request)?),
        };
        let adapter =
            interfaces::choose(self.interface, loopback).map_err(ApiError::bad_request)?;
        let endpoint = Endpoint {
            interface: adapter.ip,
            broadcast: adapter.broadcast(),
            port: self.port.unwrap_or(BACNET_PORT),
            bbmd,
        };
        Ok((adapter, endpoint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(port: u16) -> Endpoint {
        Endpoint {
            interface: Ipv4Addr::LOCALHOST,
            broadcast: Ipv4Addr::new(127, 255, 255, 255),
            port,
            bbmd: None,
        }
    }

    #[test]
    fn a_device_is_reached_only_over_the_link_it_was_written_on() {
        let station = Some(Address::Mstp(5));
        let ip = Some(Address::Ip("10.0.0.20:47808".parse().unwrap()));
        assert!(
            other_link(2005, station, false)
                .unwrap()
                .contains("sits on an MS/TP trunk as station 5")
        );
        assert_eq!(other_link(2005, station, true), None);
        assert!(
            other_link(3001, ip, true)
                .unwrap()
                .contains("is on BACnet/IP")
        );
        assert_eq!(other_link(3001, ip, false), None);
        // behind a router no address is kept: the open connection is tried
        assert_eq!(other_link(3001, None, true), None);
    }

    /// The tasks that follow a connection must not keep it open: a search on another adapter or
    /// port has to close it ("a request is still running" otherwise, field test 2 November).
    #[tokio::test]
    async fn a_search_elsewhere_replaces_the_connection() {
        let state = Arc::new(AppState::default());
        state.connect(&endpoint(47_891)).await.unwrap();
        tokio::task::yield_now().await;
        state.connect(&endpoint(47_892)).await.unwrap();
    }
}

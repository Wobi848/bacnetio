//! Finding devices: a Who-Is broadcast first, then each address of the subnet asked directly.
//! Direct questions get through a firewall that drops broadcast answers (concept section 7.4).

use std::collections::BTreeMap;
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use bacnet_client::client::DiscoveredRouter;
use bacnet_services::who_is::DeviceInstanceRange;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

use bacnet_types::primitives::ObjectIdentifier;

use crate::connection::{Client, mac, socket_address};

/// Highest device instance a device can have; the next one is the wildcard.
pub const MAX_INSTANCE: u32 = 4_194_302;
/// Parallel reads of device details; small controllers and routers get few at a time.
const DETAILS_PARALLEL: usize = 8;
/// Parallel reads of object lists, apart from the details so that names never wait for a long list.
const LISTS_PARALLEL: usize = 8;
/// How long to wait for answers to Who-Is.
pub const DEFAULT_WHO_IS_WAIT: Duration = Duration::from_secs(3);
/// How often a running scan looks for new answers to Who-Is.
const LIVE_POLL: Duration = Duration::from_millis(200);

mod details;
mod live;
mod locate;
mod objects;
mod sweep;
pub use sweep::WILDCARD_INSTANCE;

use live::Live;
pub use locate::{aim_at, ask_again, find_one, find_routed, locate, parse_station};
pub use objects::read_object_list;

/// Where a device behind a BACnet router sits.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteStation {
    pub network: u16,
    pub mac: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Details {
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    /// Why some details could not be read.
    pub error: Option<String>,
}

/// A device's object list as far as the search has read it: identifiers only, no names or values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectList {
    /// Entries known so far, and all of them once the device told.
    pub read: usize,
    pub total: Option<usize>,
    /// The whole list when it is read.
    pub objects: Option<Vec<ObjectIdentifier>>,
    /// It had to be read in parts, which the reading of the points reports.
    pub in_parts: bool,
    /// Why it could not be read. The search goes on without it.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub instance: u32,
    /// The device itself, or the router in front of it.
    pub address: SocketAddrV4,
    pub remote: Option<RemoteStation>,
    pub answered_who_is: bool,
    pub details: Details,
    pub objects: ObjectList,
}

/// A BACnet router on the local network and the networks it reaches.
#[derive(Debug, Clone, PartialEq)]
pub struct Router {
    pub address: SocketAddrV4,
    pub networks: Vec<u16>,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub swept: usize,
    pub devices: Vec<Device>,
    /// Routers that answered Who-Is-Router-To-Network.
    pub routers: Vec<Router>,
    /// Device instances that answered from more than one address.
    pub duplicates: BTreeMap<u32, Vec<SocketAddrV4>>,
    /// Addresses that answered the direct question with an error: BACnet is there, but the
    /// device does not accept the wildcard instance.
    pub refused: Vec<(SocketAddrV4, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    WhoIs,
    Sweep { asked: usize, total: usize },
    Details,
    ObjectLists,
}

/// How far a scan is, with the devices found so far, for showing them while it runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub phase: Phase,
    pub found: usize,
    pub devices: Vec<Device>,
}

impl Progress {
    pub fn start() -> Self {
        Progress {
            phase: Phase::WhoIs,
            found: 0,
            devices: Vec::new(),
        }
    }
}

/// Who-Is, and at the same time a direct question to every `sweep` address.
pub async fn run(
    client: &Arc<Client>,
    who_is_wait: Duration,
    sweep: Vec<SocketAddrV4>,
    progress: &(dyn Fn(Progress) + Sync),
) -> anyhow::Result<ScanReport> {
    run_in_range(client, who_is_wait, sweep, None, false, progress).await
}

/// Like [`run`], but only devices with an ID in `instances` (from, to) are asked by Who-Is and
/// listed. On large sites this keeps the answers to Who-Is from flooding the network.
///
/// Every device shows in `progress` the moment it answers, and its name is read at once; the
/// sweep for devices behind a firewall runs while the answers to Who-Is come in. With `read_lists`,
/// each device's object list follows its name, and the scan ends when all are read or have failed.
pub async fn run_in_range(
    client: &Arc<Client>,
    who_is_wait: Duration,
    sweep: Vec<SocketAddrV4>,
    instances: Option<(u32, u32)>,
    read_lists: bool,
    progress: &(dyn Fn(Progress) + Sync),
) -> anyhow::Result<ScanReport> {
    let range = instances
        .map(|(from, to)| {
            DeviceInstanceRange::new(from, to)
                .with_context(|| format!("{from}–{to} is not a valid device ID range"))
        })
        .transpose()?;
    let wanted = |instance: u32| instances.is_none_or(|(from, to)| (from..=to).contains(&instance));
    let mut live = Live::new(sweep.len());
    progress(live.progress(Phase::WhoIs, wanted));
    // the client keeps the first address it heard for an ID and refuses others: a device that took
    // a new ID, or several sharing one, would stay where the last search found them
    client.clear_devices().await;
    client.who_is(range).await.context("cannot send Who-Is")?;
    let found_routers = listen(
        client,
        who_is_wait,
        sweep,
        &mut live,
        read_lists,
        &wanted,
        progress,
    )
    .await?;
    Ok(report(live, found_routers, wanted))
}

/// Takes in answers to Who-Is, the sweep and the details read for each device until all three
/// are done, and shows the devices found so far after each answer.
async fn listen(
    client: &Arc<Client>,
    who_is_wait: Duration,
    sweep: Vec<SocketAddrV4>,
    live: &mut Live,
    read_lists: bool,
    wanted: &impl Fn(u32) -> bool,
    progress: &(dyn Fn(Progress) + Sync),
) -> anyhow::Result<Vec<DiscoveredRouter>> {
    let (answers_sender, mut answers) = mpsc::unbounded_channel();
    tokio::spawn(sweep::ask_all(Arc::clone(client), sweep, answers_sender));
    // the routers answer while the devices do, so asking them costs no extra time
    let routers = client.discover_routers(None, who_is_wait);
    tokio::pin!(routers);
    let permits = Arc::new(Semaphore::new(DETAILS_PARALLEL));
    let mut details: JoinSet<(u32, Details)> = JoinSet::new();
    let list_permits = Arc::new(Semaphore::new(LISTS_PARALLEL));
    let mut lists: JoinSet<(u32, ObjectList)> = JoinSet::new();
    let (listed_sender, mut listed) = mpsc::unbounded_channel::<objects::Listed>();
    let mut found_routers = None;
    let mut sweeping = true;
    let mut tick = tokio::time::interval(LIVE_POLL);
    loop {
        tokio::select! {
            found = &mut routers, if found_routers.is_none() => {
                found_routers = Some(found.context("cannot send Who-Is-Router-To-Network")?);
                live.merge(client.discovered_devices().await);
            }
            answer = answers.recv(), if sweeping => match answer {
                Some((address, answer)) => live.answer(address, answer),
                None => sweeping = false,
            },
            Some(joined) = details.join_next() => {
                let (instance, found) = joined.expect("details task does not panic");
                // a device that told nothing about itself would only run into more timeouts
                let silent = found.error.clone().filter(|_| found.name.is_none());
                live.details(instance, found);
                match silent {
                    _ if !read_lists => {}
                    Some(reason) => live.object_list(instance, ObjectList::failed(reason)),
                    None => {
                        let (client, permits, listed) =
                            (Arc::clone(client), Arc::clone(&list_permits), listed_sender.clone());
                        lists.spawn(async move {
                            let _permit = permits
                                .acquire_owned()
                                .await
                                .expect("semaphore is never closed");
                            (instance, read_object_list(&client, instance, &listed).await)
                        });
                    }
                }
            }
            Some(joined) = lists.join_next() => {
                let (instance, list) = joined.expect("list task does not panic");
                live.object_list(instance, list);
            }
            Some((instance, read, total)) = listed.recv() => live.listing(instance, read, total),
            _ = tick.tick() => live.merge(client.discovered_devices().await),
        }
        for device in live.undetailed() {
            if spawn_details(client, &permits, &mut details, &device).await? {
                live.added(&device);
            }
        }
        let phase = match (&found_routers, sweeping) {
            (None, _) => Phase::WhoIs,
            (Some(_), true) => Phase::Sweep {
                asked: live.asked,
                total: live.total,
            },
            (Some(routers), false) if details.is_empty() && lists.is_empty() => {
                return Ok(routers.clone());
            }
            (Some(_), false) if details.is_empty() => Phase::ObjectLists,
            (Some(_), false) => Phase::Details,
        };
        progress(live.progress(phase, wanted));
    }
}

/// The devices in the wanted range, sorted, with duplicates and routers.
fn report(live: Live, routers: Vec<DiscoveredRouter>, wanted: impl Fn(u32) -> bool) -> ScanReport {
    let mut devices = live.devices;
    devices.retain(|device| wanted(device.instance));
    devices.sort_by_key(|device| (device.instance, device.address));
    ScanReport {
        swept: live.total,
        duplicates: duplicates(&devices),
        devices,
        routers: routers
            .into_iter()
            .filter_map(|router| {
                Some(Router {
                    address: socket_address(&router.mac)?,
                    networks: router.networks,
                })
            })
            .collect(),
        refused: live.refused,
    }
}

/// Reads a device's name, vendor and model in the background. A device found only by asking
/// it directly is made known to the client first, so that requests by its ID reach it; true
/// when that happened.
async fn spawn_details(
    client: &Arc<Client>,
    permits: &Arc<Semaphore>,
    details: &mut JoinSet<(u32, Details)>,
    device: &Device,
) -> anyhow::Result<bool> {
    let add = !device.answered_who_is && client.get_device(device.instance).await.is_none();
    if add {
        client
            .add_device(device.instance, &mac(device.address))
            .await
            .with_context(|| format!("cannot remember device {}", device.instance))?;
    }
    let instance = device.instance;
    let client = Arc::clone(client);
    let permits = Arc::clone(permits);
    details.spawn(async move {
        let _permit = permits
            .acquire_owned()
            .await
            .expect("semaphore is never closed");
        (instance, details::device_details(&client, instance).await)
    });
    Ok(add)
}

fn duplicates(devices: &[Device]) -> BTreeMap<u32, Vec<SocketAddrV4>> {
    let mut addresses: BTreeMap<u32, Vec<SocketAddrV4>> = BTreeMap::new();
    for device in devices {
        addresses
            .entry(device.instance)
            .or_default()
            .push(device.address);
    }
    addresses.retain(|_, found_at| found_at.len() > 1);
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::BACNET_PORT;
    use std::net::Ipv4Addr;

    #[test]
    fn duplicates_only_lists_instances_seen_twice() {
        let device = |instance, last| Device {
            instance,
            address: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, last), BACNET_PORT),
            remote: None,
            answered_who_is: false,
            details: Details::default(),
            objects: Default::default(),
        };
        let found = duplicates(&[device(1, 1), device(2, 2), device(1, 3)]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[&1].len(), 2);
    }
}

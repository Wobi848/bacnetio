//! Finding one device: by its address, or by Who-Is for its ID.

use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use bacnet_client::discovery::RoutedDeviceConfig;
use bacnet_services::who_is::DeviceInstanceRange;
use bacnet_types::enums::Segmentation;

use super::{Device, ObjectList, RemoteStation, details, sweep};
use crate::connection::{Client, mac};

/// What is assumed of a device behind a router until it says otherwise: the common size on MS/TP.
const ASSUMED_ROUTED_APDU: u32 = 480;

/// How often to look whether the device asked for has answered Who-Is.
const LOCATE_POLL: Duration = Duration::from_millis(100);

/// Finds one device, by Who-Is for its instance or by asking its address, and makes it known to
/// the client so requests by instance reach it. Returns its instance.
pub async fn locate(
    client: &Client,
    instance: Option<u32>,
    address: Option<SocketAddrV4>,
    who_is_wait: Duration,
) -> anyhow::Result<u32> {
    match (instance, address) {
        (_, Some(address)) => locate_at(client, instance, address).await,
        (Some(instance), None) => locate_by_who_is(client, instance, who_is_wait).await,
        (None, None) => bail!("give a device ID or an address"),
    }
}

/// One device at a known address, with its details, as a scan would list it.
pub async fn find_one(
    client: &Client,
    instance: Option<u32>,
    address: SocketAddrV4,
) -> anyhow::Result<Device> {
    let instance = locate_at(client, instance, address).await?;
    Ok(Device {
        instance,
        address,
        remote: None,
        answered_who_is: false,
        details: details::device_details(client, instance).await,
        objects: ObjectList::default(),
    })
}

async fn locate_at(
    client: &Client,
    instance: Option<u32>,
    address: SocketAddrV4,
) -> anyhow::Result<u32> {
    let found = match (sweep::ask_instance(client, address).await, instance) {
        (None, _) => bail!("no BACnet device answers at {address}"),
        (Some(Ok(found)), Some(expected)) if found != expected => {
            bail!("the device at {address} has ID {found}, not {expected}")
        }
        (Some(Ok(found)), _) => found,
        // it refuses the wildcard question, but the caller knows its ID
        (Some(Err(_)), Some(expected)) => expected,
        (Some(Err(reason)), None) => bail!(
            "{address} answers but refuses to tell its device ID ({reason}); add the device ID"
        ),
    };
    client
        .add_device(found, &mac(address))
        .await
        .with_context(|| format!("cannot remember device {found}"))?;
    Ok(found)
}

async fn locate_by_who_is(client: &Client, instance: u32, wait: Duration) -> anyhow::Result<u32> {
    let range = DeviceInstanceRange::new(instance, instance)
        .with_context(|| format!("{instance} is not a valid device ID"))?;
    client
        .who_is(Some(range))
        .await
        .context("cannot send Who-Is")?;
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if client.get_device(instance).await.is_some() {
            return Ok(instance);
        }
        tokio::time::sleep(LOCATE_POLL).await;
    }
    bail!(
        "device {instance} did not answer Who-Is within {} s; it may answer only when asked at \
         its address",
        wait.as_secs()
    )
}

/// Asks one device to announce itself again, without waiting for the answer.
pub async fn ask_again(client: &Client, instance: u32) {
    let Ok(range) = DeviceInstanceRange::new(instance, instance) else {
        return;
    };
    if let Err(reason) = client.who_is(Some(range)).await {
        eprintln!("Warning: cannot ask device {instance} again: {reason}");
    }
}

/// A station's address on its network: a plain number (MS/TP), or bytes in hex with `:` between.
pub fn parse_station(text: &str) -> Result<Vec<u8>, String> {
    let invalid = || format!("\"{text}\" is not a station address (such as 12 or 00:1a)");
    if text.contains(':') {
        return text
            .split(':')
            .map(|byte| u8::from_str_radix(byte, 16).map_err(|_| invalid()))
            .collect();
    }
    text.parse::<u8>()
        .map(|station| vec![station])
        .map_err(|_| invalid())
}

/// A device behind the router at `router`, on `network`, with its details.
pub async fn find_routed(
    client: &Client,
    instance: u32,
    router: SocketAddrV4,
    network: u16,
    station: Vec<u8>,
) -> anyhow::Result<Device> {
    remember_routed(client, instance, router, network, &station).await?;
    let details = details::device_details(client, instance).await;
    if details.name.is_none()
        && let Some(reason) = &details.error
    {
        bail!("device {instance} does not answer through {router}: {reason}");
    }
    Ok(Device {
        instance,
        address: router,
        remote: Some(RemoteStation {
            network,
            mac: station,
        }),
        answered_who_is: false,
        details,
        objects: ObjectList::default(),
    })
}

async fn remember_routed(
    client: &Client,
    instance: u32,
    router: SocketAddrV4,
    network: u16,
    station: &[u8],
) -> anyhow::Result<()> {
    client
        .add_routed_device(RoutedDeviceConfig {
            instance,
            router_mac: mac(router).to_vec(),
            remote_network: network,
            remote_mac: station.to_vec(),
            max_apdu_length: ASSUMED_ROUTED_APDU,
            segmentation_supported: Segmentation::NONE,
            max_segments_accepted: None,
        })
        .await
        .with_context(|| format!("cannot reach device {instance} through {router}"))
}

/// Makes requests by `device.instance` go to this very device. Where several devices share an ID,
/// the client knows one of them only, the one it heard first, not always the one in the list
/// (field test 3 November: a new ID went to another actuator than the one shown).
pub async fn aim_at(client: &Client, device: &Device) -> anyhow::Result<()> {
    let known = client.get_device(device.instance).await;
    let already = known.is_some_and(|known| match &device.remote {
        Some(remote) => {
            known.source_network == Some(remote.network)
                && known
                    .source_address
                    .as_ref()
                    .map(|station| station.as_slice())
                    == Some(remote.mac.as_slice())
                && known.mac_address.as_slice() == mac(device.address)
        }
        None => {
            known.source_network.is_none() && known.mac_address.as_slice() == mac(device.address)
        }
    });
    if already {
        return Ok(());
    }
    match &device.remote {
        Some(remote) => {
            remember_routed(
                client,
                device.instance,
                device.address,
                remote.network,
                &remote.mac,
            )
            .await
        }
        None => client
            .add_device(device.instance, &mac(device.address))
            .await
            .with_context(|| format!("cannot remember device {}", device.instance)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_station;

    #[test]
    fn a_station_is_a_number_or_bytes_in_hex() {
        assert_eq!(parse_station("12"), Ok(vec![12]));
        assert_eq!(parse_station("00:1a"), Ok(vec![0, 0x1a]));
        assert!(parse_station("300").is_err());
        assert!(parse_station("zz:1").is_err());
    }
}

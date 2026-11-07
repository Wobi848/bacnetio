//! The BACnet/IP client every command talks through: opening it on the chosen adapter, and the
//! small conversions requests need.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use bacnet_client::client::BACnetClient;
use bacnet_encoding::primitives::decode_application_value;
use bacnet_transport::any::AnyTransport;
use bacnet_transport::bip::{BipMode, ForeignDeviceConfig, ForeignRegistrationOutcome};
use bacnet_types::enums::{AbortReason, PropertyIdentifier};
use bacnet_types::error::{Error, RemoteError};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

pub const BACNET_PORT: u16 = 47808;
const APDU_TIMEOUT_MS: u64 = 2000;
const APDU_RETRIES: u8 = 1;
/// Lifetime of the registration at a BBMD; renewed automatically at half of it.
const FOREIGN_DEVICE_TTL_S: u16 = 300;
const REGISTRATION_WAIT: Duration = Duration::from_secs(5);
const REGISTRATION_POLL: Duration = Duration::from_millis(100);

/// Over any transport, so that a connection to an MS/TP trunk can take BACnet/IP's place
/// (`docs/concept-mstp.md`).
pub type Client = BACnetClient<AnyTransport>;

pub struct Endpoint {
    pub interface: Ipv4Addr,
    pub broadcast: Ipv4Addr,
    /// Local port; [`BACNET_PORT`] to hear broadcast answers.
    pub port: u16,
    /// BBMD to register with as a foreign device, to reach devices in other subnets.
    pub bbmd: Option<SocketAddrV4>,
}

pub struct Connection {
    pub client: Arc<Client>,
    pub local_port: u16,
    /// Why the requested port could not be used; another one was taken instead.
    pub port_unavailable: Option<String>,
}

/// Opens the client on the requested port, or on a port the system picks if that one is taken.
pub async fn open(endpoint: &Endpoint) -> anyhow::Result<Connection> {
    if endpoint.port != 0
        && let Some(reason) = taken_on_interface(endpoint.interface, endpoint.port)
    {
        let client = start(endpoint, 0)
            .await
            .context("cannot open a UDP socket")?;
        return finish(endpoint, client, 0, Some(reason)).await;
    }
    let (client, local_port, port_unavailable) = match start(endpoint, endpoint.port).await {
        Ok(client) => (client, endpoint.port, None),
        Err(error) if endpoint.port != 0 => {
            let client = start(endpoint, 0)
                .await
                .context("cannot open a UDP socket")?;
            (client, 0, Some(error.to_string()))
        }
        Err(error) => return Err(error).context("cannot open a UDP socket"),
    };
    finish(endpoint, client, local_port, port_unavailable).await
}

/// Another program bound to the adapter's own address on the port, such as a BACnet browser
/// running as a service. Our socket on 0.0.0.0 would still open beside it, but the system hands
/// the devices' answers to the more specific socket, so nothing would arrive.
fn taken_on_interface(interface: Ipv4Addr, port: u16) -> Option<String> {
    match std::net::UdpSocket::bind(SocketAddrV4::new(interface, port)) {
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            Some(format!("another program uses {interface}:{port}"))
        }
        _ => None,
    }
}

/// Where bacnetio joins an MS/TP trunk (docs/concept-mstp.md): the serial port, the trunk's
/// speed, bacnetio's own station address and the highest master address to poll for.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Trunk {
    pub port: String,
    pub baud: u32,
    pub station: u8,
    pub max_master: u8,
}

/// On MS/TP an answer waits for the token: a ring of many stations takes a while to come round.
const MSTP_APDU_TIMEOUT_MS: u64 = 3000;
/// The largest APDU an MS/TP frame carries (Clause 9.3).
const MSTP_MAX_APDU: u16 = 480;
/// How long a master may take to poll for bacnetio's address and pass it the token.
const RING_WAIT: Duration = Duration::from_secs(20);
const RING_POLL: Duration = Duration::from_millis(100);

/// Opens the serial port and joins the trunk as a master station.
#[cfg(not(target_os = "android"))]
pub async fn open_mstp(trunk: &Trunk) -> anyhow::Result<Connection> {
    use bacnet_transport::mstp_serial::{SerialConfig, TokioSerialPort};
    let serial = TokioSerialPort::open(&SerialConfig {
        port_name: trunk.port.clone(),
        baud_rate: trunk.baud,
    })
    .with_context(|| format!("cannot open {}", trunk.port))?;
    open_mstp_on(serial, trunk).await
}

/// Joins the trunk on an open serial line; [`open_mstp`] with a real port.
pub async fn open_mstp_on<S: bacnet_transport::mstp::SerialPort>(
    serial: S,
    trunk: &Trunk,
) -> anyhow::Result<Connection> {
    use bacnet_transport::mstp::{MstpConfig, MstpTransport};
    let config = MstpConfig {
        this_station: trunk.station,
        max_master: trunk.max_master,
        baud_rate: trunk.baud,
        ..MstpConfig::default()
    };
    let transport: AnyTransport = MstpTransport::new(serial, config).into();
    let client = BACnetClient::generic_builder()
        .transport(transport)
        .apdu_timeout_ms(MSTP_APDU_TIMEOUT_MS)
        .apdu_retries(APDU_RETRIES)
        .max_apdu_length(MSTP_MAX_APDU)
        .build()
        .await
        .with_context(|| format!("cannot join the MS/TP trunk on {}", trunk.port))?;
    // a Who-Is sent before bacnetio holds the token would wait in its queue, and the search
    // would end before the answers come
    if let Err(error) = wait_in_ring(&client, trunk).await {
        let mut client = client;
        client
            .stop()
            .await
            .context("cannot close the serial port")?;
        return Err(error);
    }
    Ok(Connection {
        client: Arc::new(client),
        local_port: 0,
        port_unavailable: None,
    })
}

/// Waits until bacnetio has held the token: passed it on in a ring, or took it alone on a quiet
/// trunk.
async fn wait_in_ring(client: &Client, trunk: &Trunk) -> anyhow::Result<()> {
    let AnyTransport::Mstp(transport) = client.transport() else {
        return Ok(());
    };
    let diagnostics = transport.diagnostics();
    let deadline = Instant::now() + RING_WAIT;
    loop {
        let alone = diagnostics
            .node_state()
            .is_some_and(|node| node.sole_master);
        if alone || diagnostics.snapshot().mac.token_rotations > 0 {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!(
                "no master took bacnetio into the token ring as station {} within {} s: check \
                 the speed, and that the station is inside the masters' Max_Master",
                trunk.station,
                RING_WAIT.as_secs()
            );
        }
        tokio::time::sleep(RING_POLL).await;
    }
}

/// Registers with the BBMD where one is given.
async fn finish(
    endpoint: &Endpoint,
    client: Client,
    local_port: u16,
    port_unavailable: Option<String>,
) -> anyhow::Result<Connection> {
    if let Some(bbmd) = endpoint.bbmd
        && let Err(error) = wait_for_registration(&client, bbmd).await
    {
        let mut client = client;
        client.stop().await.context("cannot close the UDP socket")?;
        return Err(error);
    }
    Ok(Connection {
        client: Arc::new(client),
        local_port,
        port_unavailable,
    })
}

impl Connection {
    /// Fails while a request still holds the client.
    pub async fn close(self) -> anyhow::Result<()> {
        let Some(mut client) = Arc::into_inner(self.client) else {
            bail!("a request is still running");
        };
        client.stop().await.context("cannot close the UDP socket")
    }
}

async fn start(endpoint: &Endpoint, port: u16) -> Result<Client, Error> {
    let builder = BACnetClient::bip_builder()
        .interface(endpoint.interface)
        .port(port)
        .broadcast_address(endpoint.broadcast)
        .apdu_timeout_ms(APDU_TIMEOUT_MS)
        .apdu_retries(APDU_RETRIES);
    let builder = match endpoint.bbmd {
        Some(bbmd) => builder.mode(BipMode::Foreign(ForeignDeviceConfig {
            bbmd_ip: *bbmd.ip(),
            bbmd_port: bbmd.port(),
            ttl: FOREIGN_DEVICE_TTL_S,
            renewal_interval: None,
        })),
        None => builder,
    };
    builder.build_any().await
}

/// Waits until the BBMD has accepted the registration, so the first Who-Is reaches its subnet.
async fn wait_for_registration(client: &Client, bbmd: SocketAddrV4) -> anyhow::Result<()> {
    let deadline = Instant::now() + REGISTRATION_WAIT;
    loop {
        let registration = client
            .foreign_registration()
            .context("cannot read the BBMD registration")?;
        match registration.last_outcome {
            Some(ForeignRegistrationOutcome::Accepted) => return Ok(()),
            Some(ForeignRegistrationOutcome::Refused) => bail!(
                "the BBMD at {bbmd} refused the registration: it does not accept foreign \
                 devices, or its table is full"
            ),
            Some(ForeignRegistrationOutcome::TimedOut) => bail!(
                "the BBMD at {bbmd} does not answer: check address and port, and whether it is \
                 a BBMD at all"
            ),
            Some(ForeignRegistrationOutcome::LocalError) => {
                bail!("cannot send the registration to the BBMD at {bbmd}")
            }
            _ => {}
        }
        if Instant::now() >= deadline {
            bail!(
                "the BBMD at {bbmd} did not answer the registration within {} s",
                REGISTRATION_WAIT.as_secs()
            );
        }
        tokio::time::sleep(REGISTRATION_POLL).await;
    }
}

#[derive(Debug)]
pub enum ReadError {
    /// No answer, or an Error, Reject or Abort from the device.
    Request(Error),
    /// The answer could not be decoded.
    Decode(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Request(error) => write!(f, "{error}"),
            ReadError::Decode(reason) => write!(f, "unreadable answer: {reason}"),
        }
    }
}

/// Reads one property of an object on a device the client knows, decoded.
pub async fn read_value(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
    property: PropertyIdentifier,
    array_index: Option<u32>,
) -> Result<PropertyValue, ReadError> {
    let ack = client
        .read_property_from_device(device, object, property, array_index)
        .await
        .map_err(ReadError::Request)?;
    decode_value(&ack.property_value).map_err(ReadError::Decode)
}

/// An address as typed: IP, or IP:PORT when not the BACnet port.
pub fn parse_address(text: &str) -> Result<SocketAddrV4, String> {
    if let Ok(address) = text.parse::<SocketAddrV4>() {
        return Ok(address);
    }
    text.parse::<Ipv4Addr>()
        .map(|ip| SocketAddrV4::new(ip, BACNET_PORT))
        .map_err(|_| format!("{text} is not an IP address or IP:PORT"))
}

/// No answer at all, as opposed to an answer that refuses.
pub fn is_timeout(error: &Error) -> bool {
    matches!(
        error,
        Error::Remote(RemoteError::Abort {
            reason: AbortReason::TSM_TIMEOUT
        })
    )
}

pub fn decode_value(bytes: &[u8]) -> Result<PropertyValue, String> {
    decode_application_value(bytes, 0)
        .map(|(value, _)| value)
        .map_err(|error| error.to_string())
}

/// A whole array or list, such as the object list or the priority array.
pub fn decode_values(bytes: &[u8]) -> Result<Vec<PropertyValue>, String> {
    let mut values = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (value, next) =
            decode_application_value(bytes, offset).map_err(|error| error.to_string())?;
        values.push(value);
        offset = next;
    }
    Ok(values)
}

/// BACnet/IP MAC: four address bytes and the port, big-endian.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_local_tsm_timeout_counts_as_no_answer() {
        assert!(is_timeout(&Error::Remote(RemoteError::Abort {
            reason: AbortReason::TSM_TIMEOUT
        })));
        assert!(!is_timeout(&Error::Remote(RemoteError::Abort {
            reason: AbortReason::SEGMENTATION_NOT_SUPPORTED
        })));
    }
}

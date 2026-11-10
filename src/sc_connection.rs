//! The connection to the hub of a BACnet/SC network (docs/concept-sc.md), into the same client as
//! BACnet/IP and MS/TP, and why it failed in words.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use bacnet_client::client::BACnetClient;
use bacnet_transport::any::AnyTransport;
use bacnet_transport::sc::{ScActiveHub, ScConnectionState, ScReconnectConfig, ScTransport};
use bacnet_transport::sc_tls::{ScNodeTlsConfig, TlsWebSocket};

use crate::connection::{APDU_RETRIES, APDU_TIMEOUT_MS, Client, Connection};
use crate::sc_identity::Identity;

/// How long the TLS and WebSocket handshake with an SC hub may take.
const SC_CONNECT_WAIT: Duration = Duration::from_secs(10);

/// After the connection drops: again after 1 s, then less often up to every 30 s, for as long as
/// bacnetio is on the network; with a failover hub the library turns to it when the primary stays
/// away, and back when it returns (Annex AB.5.2).
const RECONNECT: ScReconnectConfig = ScReconnectConfig {
    initial_delay_ms: 1_000,
    max_delay_ms: 30_000,
    max_retries: u32::MAX,
};

/// The hubs of an SC network: the primary, and the failover hub where the site has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hubs {
    pub primary: String,
    pub failover: Option<String>,
}

/// The state of the connection to the hub, for the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct HubStatus {
    /// Connected now; otherwise bacnetio is connecting again by itself.
    pub connected: bool,
    /// On the failover hub, or dialling it.
    pub failover: bool,
}

/// Connects to the hub of a BACnet/SC network (docs/concept-sc.md) as `identity`, with the
/// certificates in `tls`: the primary hub, or the failover hub when the primary does not answer.
/// The reason given is the primary's.
pub async fn open_sc(
    hubs: &Hubs,
    tls: ScNodeTlsConfig,
    identity: Identity,
) -> anyhow::Result<Connection> {
    let (socket, initial) = match dial(&hubs.primary, &tls).await {
        Ok(socket) => (socket, ScActiveHub::Primary),
        Err(primary) => match &hubs.failover {
            Some(failover) => match dial(failover, &tls).await {
                Ok(socket) => (socket, ScActiveHub::Failover),
                Err(_) => return Err(primary),
            },
            None => return Err(primary),
        },
    };
    let mut transport = ScTransport::new(socket, identity.vmac)
        .with_device_uuid(identity.uuid)
        .with_initial_hub(initial)
        .with_reconnect(RECONNECT)
        .with_connector(connector(&hubs.primary, &tls));
    if let Some(failover) = &hubs.failover {
        transport = transport.with_failover_connector(connector(failover, &tls));
    }
    let client = BACnetClient::generic_builder()
        .transport(AnyTransport::Sc(Box::new(transport)))
        .apdu_timeout_ms(APDU_TIMEOUT_MS)
        .apdu_retries(APDU_RETRIES)
        .build()
        .await
        .with_context(|| format!("the hub at {} did not take bacnetio in", hubs.primary))?;
    Ok(Connection {
        client: Arc::new(client),
        local_port: 0,
        port_unavailable: None,
    })
}

/// The TLS and WebSocket connection to one hub, or why it failed in words.
async fn dial(hub: &str, tls: &ScNodeTlsConfig) -> anyhow::Result<TlsWebSocket> {
    tokio::time::timeout(SC_CONNECT_WAIT, TlsWebSocket::connect(hub, tls.clone()))
        .await
        .with_context(|| {
            format!(
                "the hub at {hub} did not answer within {} s",
                SC_CONNECT_WAIT.as_secs()
            )
        })?
        .map_err(|error| anyhow::anyhow!(hub_failure(hub, &error.to_string())))
}

/// A fresh connection to `hub` for every attempt to connect again.
fn connector(
    hub: &str,
    tls: &ScNodeTlsConfig,
) -> impl Fn() -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<TlsWebSocket, bacnet_types::error::Error>> + Send>,
> + Send
+ Sync
+ 'static {
    let (hub, tls) = (hub.to_string(), tls.clone());
    move || {
        let (hub, tls) = (hub.clone(), tls.clone());
        Box::pin(async move { TlsWebSocket::connect(&hub, tls).await })
    }
}

/// The state of the hub connection of an SC client; `None` on another link.
pub fn hub_status(client: &Client) -> Option<HubStatus> {
    let AnyTransport::Sc(transport) = client.transport() else {
        return None;
    };
    let status = transport.connection_status();
    Some(HubStatus {
        connected: status.state == ScConnectionState::Connected,
        failover: status.active_hub == Some(ScActiveHub::Failover),
    })
}

/// Why the hub could not be reached, in words, by the stage the connection failed at.
fn hub_failure(hub: &str, error: &str) -> String {
    if error.contains("host-lookup failure") {
        format!("the name of the hub at {hub} is not known: check the address")
    } else if error.contains("tcp-dial failure") {
        format!(
            "no hub answers at {hub}: check the address and that this computer reaches it (VPN, \
             firewall, port)"
        )
    } else if error.contains("not valid for name") {
        format!(
            "the certificate of the hub at {hub} is made out to another name: use the hub's name \
             as the integrator gives it"
        )
    } else if error.contains("invalid peer certificate") {
        "the hub's certificate is not signed by the site CA you chose: wrong CA file, or not the \
         hub of this site"
            .to_string()
    } else if error.contains("received fatal alert") {
        "the hub refuses bacnetio's certificate: it is not from this site's CA, has run out, or \
         the hub does not admit it yet"
            .to_string()
    } else {
        format!("cannot connect to the hub at {hub}: {error}")
    }
}

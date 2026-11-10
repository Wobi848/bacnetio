//! The connection to the hub of a BACnet/SC network (docs/concept-sc.md), into the same client as
//! BACnet/IP and MS/TP, and why it failed in words.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use bacnet_client::client::BACnetClient;
use bacnet_transport::any::AnyTransport;
use bacnet_transport::sc::ScTransport;
use bacnet_transport::sc_tls::{ScNodeTlsConfig, TlsWebSocket};

use crate::connection::{APDU_RETRIES, APDU_TIMEOUT_MS, Connection};
use crate::sc_identity::Identity;

/// How long the TLS and WebSocket handshake with an SC hub may take.
const SC_CONNECT_WAIT: Duration = Duration::from_secs(10);

/// Connects to the hub of a BACnet/SC network (docs/concept-sc.md) as `identity`, with the
/// certificates in `tls`. `hub` is the hub's `wss://` URL.
pub async fn open_sc(
    hub: &str,
    tls: ScNodeTlsConfig,
    identity: Identity,
) -> anyhow::Result<Connection> {
    let socket = tokio::time::timeout(SC_CONNECT_WAIT, TlsWebSocket::connect(hub, tls))
        .await
        .with_context(|| {
            format!(
                "the hub at {hub} did not answer within {} s",
                SC_CONNECT_WAIT.as_secs()
            )
        })?
        .map_err(|error| anyhow::anyhow!(hub_failure(hub, &error.to_string())))?;
    let transport = ScTransport::new(socket, identity.vmac).with_device_uuid(identity.uuid);
    let client = BACnetClient::generic_builder()
        .transport(AnyTransport::Sc(Box::new(transport)))
        .apdu_timeout_ms(APDU_TIMEOUT_MS)
        .apdu_retries(APDU_RETRIES)
        .build()
        .await
        .with_context(|| format!("the hub at {hub} did not take bacnetio in"))?;
    Ok(Connection {
        client: Arc::new(client),
        local_port: 0,
        port_unavailable: None,
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

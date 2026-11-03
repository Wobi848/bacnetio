//! Asking addresses directly for their device ID: gets through a firewall that drops answers to
//! broadcasts.

use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;

use bacnet_types::enums::{ObjectType, PropertyIdentifier};
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::connection::{Client, decode_value, is_timeout, mac};
use crate::present::value_kind_text;

/// Parallel direct questions while sweeping. Asking an address nobody holds leaves its packet
/// waiting for ARP for about 3 s. Too many of those fill the one socket's send buffer, and then the
/// names and object lists of the devices already found wait too.
const SWEEP_PARALLEL: usize = 64;
/// How long one address gets to answer. A sweep asks everyone, so it cannot wait for the client's
/// timeout and retries at each address nobody holds.
const SWEEP_ANSWER_WAIT: Duration = Duration::from_millis(1500);
/// Device instance every device must accept as its own (Clause 15.5.2).
pub const WILDCARD_INSTANCE: u32 = 4_194_303;

/// What one address said: nothing, its device ID, or an error that shows BACnet is there.
pub(super) type Answer = (SocketAddrV4, Option<Result<u32, String>>);

/// Asks every target for its device instance and sends each answer as it comes.
pub(super) async fn ask_all(
    client: Arc<Client>,
    targets: Vec<SocketAddrV4>,
    answers: mpsc::UnboundedSender<Answer>,
) {
    let permits = Arc::new(Semaphore::new(SWEEP_PARALLEL));
    let mut tasks = JoinSet::new();
    for target in targets {
        let client = Arc::clone(&client);
        let permits = Arc::clone(&permits);
        tasks.spawn(async move {
            let _permit = permits
                .acquire_owned()
                .await
                .expect("semaphore is never closed");
            let answer = timeout(SWEEP_ANSWER_WAIT, ask_instance(&client, target)).await;
            (target, answer.unwrap_or(None))
        });
    }
    while let Some(joined) = tasks.join_next().await {
        // the scan stops listening only when it fails, and then the answers do not matter
        if answers
            .send(joined.expect("sweep task does not panic"))
            .is_err()
        {
            return;
        }
    }
}

pub(super) async fn ask_instance(
    client: &Client,
    target: SocketAddrV4,
) -> Option<Result<u32, String>> {
    let wildcard = ObjectIdentifier::new(ObjectType::DEVICE, WILDCARD_INSTANCE)
        .expect("wildcard instance is valid");
    match client
        .read_property(
            &mac(target),
            wildcard,
            PropertyIdentifier::OBJECT_IDENTIFIER,
            None,
        )
        .await
    {
        Ok(ack) => Some(match decode_value(&ack.property_value) {
            Ok(PropertyValue::ObjectIdentifier(oid)) => Ok(oid.instance_number()),
            Ok(other) => Err(format!(
                "the device answered {}, not its object identifier",
                value_kind_text(&other)
            )),
            Err(error) => Err(error),
        }),
        Err(error) if is_timeout(&error) => None,
        Err(error) => Some(Err(error.to_string())),
    }
}

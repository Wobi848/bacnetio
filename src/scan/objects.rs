//! The object list of a found device, read during the search: identifiers only, which is cheap, so
//! that the device's objects are known before anyone opens it (concept-explorer section 2.3).

use tokio::sync::mpsc::UnboundedSender;

use super::ObjectList;
use crate::connection::Client;
use crate::limits::{self, Limits};
use crate::object_list;
use crate::observe::Event;

/// Device, entries read so far, and all entries once the device told.
pub type Listed = (u32, usize, Option<usize>);

impl ObjectList {
    pub(super) fn failed(reason: String) -> Self {
        ObjectList {
            error: Some(reason),
            ..ObjectList::default()
        }
    }
}

/// Reads the list and reports how far it is on `listed`. A failure ends up in the result, because
/// one device without a list must not stop the search.
pub async fn read_object_list(
    client: &Client,
    instance: u32,
    listed: &UnboundedSender<Listed>,
) -> ObjectList {
    let limits = limits::read_limits(client, instance)
        .await
        .unwrap_or(Limits::ASSUMED);
    let observe = |event: Event<'_>| {
        if let Event::Listed { read, total } = event {
            // nobody listens only once the search has gone, and then nobody needs this
            let _ = listed.send((instance, read, total));
        }
    };
    match object_list::read(client, instance, limits, &observe).await {
        Ok((objects, in_parts)) => ObjectList {
            read: objects.len(),
            total: Some(objects.len()),
            objects: Some(objects),
            in_parts,
            error: None,
        },
        Err(error) => ObjectList::failed(format!("{error:#}")),
    }
}

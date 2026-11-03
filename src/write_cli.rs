//! `bacnetio write` and `bacnetio release`: one value, from the command line. Each change lands in
//! the same write log as the UI's, so the UI shows what the command line holds.

use std::path::Path;
use std::time::SystemTime;

use anyhow::Context;
use bacnet_types::primitives::ObjectIdentifier;
use bacnetio::connection::socket_address;
use bacnetio::object::{self, PRIORITIES};
use bacnetio::present;
use bacnetio::property_text;
use bacnetio::storage::{self, JOURNAL_FILE};
use bacnetio::write::{self, Action, Entry};

use crate::{DeviceArgs, find_device};

pub async fn write_value(
    target: &DeviceArgs,
    object: ObjectIdentifier,
    text: &str,
    priority: Option<u8>,
    data_dir: Option<&Path>,
) -> anyhow::Result<()> {
    let value = write::parse_value(object.object_type(), text).map_err(anyhow::Error::msg)?;
    change(target, object, priority, Action::Write(value), data_dir).await
}

pub async fn release(
    target: &DeviceArgs,
    object: ObjectIdentifier,
    priority: u8,
    data_dir: Option<&Path>,
) -> anyhow::Result<()> {
    change(target, object, Some(priority), Action::Release, data_dir).await
}

async fn change(
    target: &DeviceArgs,
    object: ObjectIdentifier,
    priority: Option<u8>,
    action: Action,
    data_dir: Option<&Path>,
) -> anyhow::Result<()> {
    if let Some(priority) = priority
        && !(1..=PRIORITIES).contains(&priority)
    {
        anyhow::bail!("priority {priority} is not between 1 and {PRIORITIES}");
    }
    let (connection, device) = find_device(target).await?;
    let client = &connection.client;
    let address = client
        .get_device(device)
        .await
        .filter(|found| found.source_network.is_none())
        .and_then(|found| socket_address(&found.mac_address));
    let result = write::perform(client, device, object, priority, &action).await;
    let entry = Entry {
        time: SystemTime::now(),
        device,
        address,
        object,
        priority,
        action: action.clone(),
        error: result.as_ref().err().map(|error| error.to_string()),
    };
    if let Some(dir) = data_dir {
        let path = dir.join(JOURNAL_FILE);
        storage::append(&path, &entry)
            .with_context(|| format!("cannot save the write log to {}", path.display()))?;
    }
    result.context("the device refused")?;
    let what = match &action {
        Action::Write(value) => format!(
            "Wrote {}",
            present::value_with_unit(object.object_type(), value, None)
        ),
        Action::Release => "Released".to_string(),
        Action::WriteProperty { property, .. } => {
            format!("Wrote {}", property_text::property_name(*property))
        }
        Action::Command { text, .. } | Action::Noted(text) => text.clone(),
        Action::SetClock { date, time } => {
            format!("Set the clock to {}", present::date_time_text(date, time))
        }
    };
    let at = priority
        .map(|p| format!(" at priority {p}"))
        .unwrap_or_default();
    println!(
        "{what}{at} on {} of device {device}.",
        present::object_text(object)
    );
    let detail = object::read_object(client, device, object).await;
    println!("Present value now: {}", present::value_text(&detail.point));
    connection.close().await
}

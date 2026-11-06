//! What a journal entry did, in words: for the log on the page and its CSV.

use crate::property_text;
use crate::write::{Action, Entry};

use super::{date_time_text, value_with_unit};

/// The kind of action ("write", "release", "property", "command", "clock") and the value it set,
/// in words; `None` for a release.
pub fn entry_action(entry: &Entry) -> (&'static str, Option<String>) {
    let object_type = entry.object.object_type();
    match &entry.action {
        Action::Write(value) => ("write", Some(value_with_unit(object_type, value, None))),
        Action::Release => ("release", None),
        Action::Command { text, .. } | Action::Noted(text) => ("command", Some(text.clone())),
        Action::WriteProperty { property, value } => (
            "property",
            Some(format!(
                "{} = {}",
                property_text::property_name(*property),
                value_with_unit(object_type, value, None),
            )),
        ),
        Action::SetClock { date, time } => ("clock", Some(date_time_text(date, time))),
    }
}

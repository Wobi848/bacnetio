//! Live values: the device reports changes itself (SubscribeCOV, Clause 13.14) instead of being
//! asked again and again. Devices that refuse a subscription are read on each request instead.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use bacnet_services::cov::COVNotificationRequest;
use bacnet_types::enums::PropertyIdentifier;
use bacnet_types::error::Error;
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

use crate::connection::{Client, ReadError, decode_value, read_value};
use crate::points::StatusFlags;

/// How long a subscription lasts at the device; it ends by itself when bacnetio is gone.
pub const SUBSCRIPTION_LIFETIME: Duration = Duration::from_secs(300);
/// Renewed this long after subscribing, well before it runs out.
const RENEW_AFTER: Duration = Duration::from_secs(240);
/// Subscriber process identifier: bacnetio's notifications all go to one place.
const PROCESS_ID: u32 = 1;

type Key = (u32, ObjectIdentifier);

#[derive(Debug, Clone, PartialEq)]
pub struct Live {
    pub value: PropertyValue,
    pub status: Option<StatusFlags>,
    pub time: SystemTime,
}

#[derive(Debug, Default)]
pub struct Watch {
    live: HashMap<Key, Live>,
    subscribed: HashMap<Key, Instant>,
    refused: HashSet<Key>,
}

impl Watch {
    /// Not subscribed yet, or due for renewal; never again after a refusal.
    pub fn needs_subscription(&self, device: u32, object: ObjectIdentifier, now: Instant) -> bool {
        let key = (device, object);
        !self.refused.contains(&key)
            && self
                .subscribed
                .get(&key)
                .is_none_or(|since| now.duration_since(*since) >= RENEW_AFTER)
    }

    pub fn subscribed(&mut self, device: u32, object: ObjectIdentifier, now: Instant) {
        self.subscribed.insert((device, object), now);
    }

    pub fn refused(&mut self, device: u32, object: ObjectIdentifier) {
        self.refused.insert((device, object));
    }

    pub fn is_refused(&self, device: u32, object: ObjectIdentifier) -> bool {
        self.refused.contains(&(device, object))
    }

    /// The last report, however old.
    pub fn reported(&self, device: u32, object: ObjectIdentifier) -> Option<&Live> {
        self.live.get(&(device, object))
    }

    /// The last reported value, while the subscription runs or the report is recent.
    pub fn live(&self, device: u32, object: ObjectIdentifier, now: Instant) -> Option<&Live> {
        let key = (device, object);
        let live = self.live.get(&key)?;
        match self.subscribed.get(&key) {
            Some(since) => (now.duration_since(*since) < SUBSCRIPTION_LIFETIME).then_some(live),
            // a device may report unasked (unconfirmed COV): live for as long as a report is recent
            None => live
                .time
                .elapsed()
                .is_ok_and(|age| age < SUBSCRIPTION_LIFETIME)
                .then_some(live),
        }
    }

    /// Forgets the last report after bacnetio changed the object itself. A device only reports a
    /// change above its COV increment, so that report may never be replaced otherwise.
    pub fn forget_report(&mut self, device: u32, object: ObjectIdentifier) {
        self.live.remove(&(device, object));
    }

    /// Takes present value and status flags from a notification.
    pub fn apply(&mut self, notification: &COVNotificationRequest, time: SystemTime) {
        let device = notification.initiating_device_identifier.instance_number();
        let key = (device, notification.monitored_object_identifier);
        let mut value = None;
        let mut status = None;
        for reported in &notification.list_of_values {
            match (reported.property_identifier, decode_value(&reported.value)) {
                (PropertyIdentifier::PRESENT_VALUE, Ok(decoded)) => value = Some(decoded),
                (PropertyIdentifier::STATUS_FLAGS, Ok(PropertyValue::BitString { data, .. })) => {
                    status = Some(StatusFlags::from_bits(&data))
                }
                _ => {}
            }
        }
        if let Some(value) = value {
            self.live.insert(
                key,
                Live {
                    value,
                    status,
                    time,
                },
            );
        }
    }
}

/// Where a value came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    /// The device's own report.
    Cov,
    /// Asked for just now.
    Read,
}

/// The present value of an object: from the device's reports while it sends them (subscribing or
/// renewing first), else read from it.
pub async fn current_value(
    watch: &Mutex<Watch>,
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> Result<(Live, Source), ReadError> {
    keep_subscribed(watch, client, device, object).await;
    let reported = watch
        .lock()
        .expect("watch lock")
        .live(device, object, Instant::now())
        .cloned();
    if let Some(live) = reported {
        return Ok((live, Source::Cov));
    }
    read_now(client, device, object).await
}

/// Subscribes to the object's reports, or renews the subscription when due. False once the device
/// has refused.
pub async fn keep_subscribed(
    watch: &Mutex<Watch>,
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> bool {
    let now = Instant::now();
    let due = watch
        .lock()
        .expect("watch lock")
        .needs_subscription(device, object, now);
    if due {
        let subscribed = subscribe(client, device, object).await;
        let mut watch = watch.lock().expect("watch lock");
        match subscribed {
            Ok(()) => watch.subscribed(device, object, now),
            // only the device's own refusal counts; a device not found yet is tried again
            Err(Error::Remote(_)) => watch.refused(device, object),
            Err(_) => {}
        }
    }
    !watch.lock().expect("watch lock").is_refused(device, object)
}

/// The value asked for now, whatever the device reports by itself.
pub async fn read_now(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> Result<(Live, Source), ReadError> {
    let value = read_value(
        client,
        device,
        object,
        PropertyIdentifier::PRESENT_VALUE,
        None,
    )
    .await?;
    let live = Live {
        value,
        status: None,
        time: SystemTime::now(),
    };
    Ok((live, Source::Read))
}

/// Asks the device to report changes of the object, without confirmation.
pub async fn subscribe(
    client: &Client,
    device: u32,
    object: ObjectIdentifier,
) -> Result<(), Error> {
    client
        .subscribe_cov_to_device(
            device,
            PROCESS_ID,
            object,
            false,
            Some(SUBSCRIPTION_LIFETIME.as_secs() as u32),
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::constructed::BACnetPropertyValue;
    use bacnet_types::enums::ObjectType;

    fn valve() -> ObjectIdentifier {
        ObjectIdentifier::new(ObjectType::ANALOG_OUTPUT, 1).unwrap()
    }

    #[test]
    fn subscriptions_are_renewed_in_time_and_not_retried_after_refusal() {
        let mut watch = Watch::default();
        let start = Instant::now();
        assert!(watch.needs_subscription(7, valve(), start));
        watch.subscribed(7, valve(), start);
        assert!(!watch.needs_subscription(7, valve(), start + Duration::from_secs(60)));
        assert!(watch.needs_subscription(7, valve(), start + RENEW_AFTER));
        watch.refused(8, valve());
        assert!(!watch.needs_subscription(8, valve(), start));
    }

    #[test]
    fn notifications_fill_the_live_value_while_subscribed() {
        let mut watch = Watch::default();
        let start = Instant::now();
        watch.subscribed(7, valve(), start);
        let notification = COVNotificationRequest {
            subscriber_process_identifier: PROCESS_ID,
            initiating_device_identifier: ObjectIdentifier::new(ObjectType::DEVICE, 7).unwrap(),
            monitored_object_identifier: valve(),
            time_remaining: 300,
            list_of_values: vec![
                BACnetPropertyValue {
                    property_identifier: PropertyIdentifier::PRESENT_VALUE,
                    property_array_index: None,
                    value: vec![0x44, 0x42, 0x28, 0x00, 0x00],
                    priority: None,
                },
                BACnetPropertyValue {
                    property_identifier: PropertyIdentifier::STATUS_FLAGS,
                    property_array_index: None,
                    value: vec![0x82, 0x04, 0x20],
                    priority: None,
                },
            ],
        };
        watch.apply(&notification, SystemTime::UNIX_EPOCH);
        let live = watch.live(7, valve(), start).unwrap();
        assert_eq!(live.value, PropertyValue::Real(42.0));
        assert!(live.status.unwrap().overridden);
        assert!(
            watch
                .live(7, valve(), start + SUBSCRIPTION_LIFETIME)
                .is_none()
        );
        // the report from before bacnetio's own write no longer counts
        watch.forget_report(7, valve());
        assert!(watch.live(7, valve(), start).is_none());
    }

    #[test]
    fn a_value_reported_unasked_is_live_while_it_is_recent() {
        let mut watch = Watch::default();
        let start = Instant::now();
        assert!(watch.live(7, valve(), start).is_none());
        let recent = COVNotificationRequest {
            subscriber_process_identifier: PROCESS_ID,
            initiating_device_identifier: ObjectIdentifier::new(ObjectType::DEVICE, 7).unwrap(),
            monitored_object_identifier: valve(),
            time_remaining: 0,
            list_of_values: vec![BACnetPropertyValue {
                property_identifier: PropertyIdentifier::PRESENT_VALUE,
                property_array_index: None,
                value: vec![0x44, 0x42, 0x28, 0x00, 0x00],
                priority: None,
            }],
        };
        watch.apply(&recent, SystemTime::now());
        assert!(watch.live(7, valve(), start).is_some());
        watch.apply(&recent, SystemTime::now() - SUBSCRIPTION_LIFETIME * 2);
        assert!(watch.live(7, valve(), start).is_none());
    }
}

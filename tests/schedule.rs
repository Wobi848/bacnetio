//! Schedules and calendars read in words.

use bacnet_encoding::constructed::encode_daily_schedule;
use bacnet_types::constructed::BACnetTimeValue;
use bacnet_types::enums::ObjectType;
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue, Time};
use bacnetio::schedule::decode_week;
use bytes::BytesMut;

fn at(hour: u8, minute: u8) -> Time {
    Time {
        hour,
        minute,
        second: 0,
        hundredths: 0,
    }
}

#[test]
fn a_week_comes_out_day_by_day_with_its_changes() {
    let object = ObjectIdentifier::new(ObjectType::SCHEDULE, 1).unwrap();
    let working_day = [
        BACnetTimeValue {
            time: at(6, 30),
            value: PropertyValue::Real(21.5),
        },
        BACnetTimeValue {
            time: at(18, 0),
            value: PropertyValue::Real(17.0),
        },
    ];
    let mut bytes = BytesMut::new();
    encode_daily_schedule(&mut bytes, &working_day).unwrap();
    encode_daily_schedule(&mut bytes, &[]).unwrap();
    let week = decode_week(object, &bytes).unwrap();
    assert_eq!(week.len(), 2);
    assert_eq!(week[0].len(), 2);
    assert!(week[0][0].time.starts_with("06:30"), "{:?}", week[0][0]);
    assert_eq!(week[0][0].value, "21.5");
    assert_eq!(week[0][1].value, "17");
    assert!(week[1].is_empty());
}

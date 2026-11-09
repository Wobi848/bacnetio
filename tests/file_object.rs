mod common;

use std::time::Duration;

use bacnet_objects::file::FileObject;
use bacnet_objects::traits::BACnetObject;
use bacnet_types::enums::{FileAccessMethod, ObjectType, Segmentation};
use bacnet_types::primitives::ObjectIdentifier;
use bacnetio::{file_object, scan};
use common::{at, connect, simulated_device};

const WAIT: Duration = Duration::from_millis(500);

#[tokio::test]
async fn reads_a_stream_file_and_a_record_file_whole() {
    // larger than one answer, so it comes in several pieces
    let program: Vec<u8> = (0..5000u32).map(|n| (n % 251) as u8).collect();
    let mut stream = FileObject::new(1, "Program.bin", "program").unwrap();
    stream.set_data(program.clone());
    let mut records = FileObject::new(2, "Parameters.csv", "parameters").unwrap();
    records.set_file_access_method(FileAccessMethod::RECORD_ACCESS.to_raw());
    records.set_records(
        (1..=40)
            .map(|n| format!("setpoint {n};21.{n}").into_bytes())
            .collect(),
    );
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(stream), Box::new(records)];
    let mut device = simulated_device(7004, "AHU-10", 47962, Segmentation::BOTH, objects).await;
    let connection = connect(47963).await;
    let instance = scan::locate(&connection.client, None, Some(at(47962).into()), WAIT)
        .await
        .unwrap();

    let file = |number| ObjectIdentifier::new(ObjectType::FILE, number).unwrap();
    let read_stream = file_object::read_file(&connection.client, instance, file(1), 1000).await;
    let read_records = file_object::read_file(&connection.client, instance, file(2), 1000).await;

    connection.close().await.unwrap();
    device.stop().await.unwrap();
    assert_eq!(read_stream.unwrap(), program);
    let text = String::from_utf8(read_records.unwrap()).unwrap();
    assert_eq!(text.lines().count(), 40);
    assert!(text.starts_with("setpoint 1;21.1\nsetpoint 2;21.2\n"));
}

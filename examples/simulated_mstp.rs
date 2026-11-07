//! A simulated MS/TP device on a pseudo-terminal, to try MS/TP without an adapter or a trunk
//! (docs/concept-mstp.md): `cargo run --example simulated_mstp -- [station]`. Linux only.
//!
//! It prints the terminal's name (`/dev/pts/7`); bacnetio opens it like a USB adapter. With
//! `--test-loopback`, set `BACNETIO_TEST_SERIAL` to that name and bacnetio lists it as a port.
//! A pseudo-terminal has no real speed: listening hears the device at any speed it tries first.

#[cfg(unix)]
mod pty {
    use bacnet_transport::mstp::{MstpConfig, MstpTransport, SerialPort};
    use bacnet_types::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
    use tokio::sync::Mutex;
    use tokio_serial::SerialStream;

    pub const SPEED: u32 = 38_400;

    /// The device's end of the pseudo-terminal, read and written at the same time.
    pub struct Port {
        reader: Mutex<ReadHalf<SerialStream>>,
        writer: Mutex<WriteHalf<SerialStream>>,
    }

    impl SerialPort for Port {
        async fn write(&self, data: &[u8]) -> Result<(), Error> {
            let mut writer = self.writer.lock().await;
            writer.write_all(data).await.map_err(Error::Transport)?;
            writer.flush().await.map_err(Error::Transport)
        }

        async fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
            self.reader
                .lock()
                .await
                .read(buf)
                .await
                .map_err(Error::Transport)
        }
    }

    /// The device's transport and the terminal's name for bacnetio. The terminal stays open
    /// here too, without a lock, so the device keeps running while no bacnetio has it open.
    /// A bacnetio that was killed cannot take back its exclusive mark on the terminal, and as the
    /// terminal stays open here, the system never clears it as it does for a USB adapter: every
    /// later opening would be refused as busy. So it is cleared here, now and then.
    pub async fn keep_openable(terminal: std::fs::File) {
        use std::os::fd::AsRawFd;
        loop {
            // SAFETY: TIOCNXCL takes no argument and only clears a flag of this open terminal
            unsafe { libc::ioctl(terminal.as_raw_fd(), libc::TIOCNXCL) };
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    pub fn open(station: u8) -> (MstpTransport<Port>, String, std::fs::File) {
        use tokio_serial::SerialPort as _;
        let (device_end, other_end) = SerialStream::pair().expect("cannot open a pseudo-terminal");
        let name = other_end.name().expect("the pseudo-terminal has no name");
        let kept_open = std::fs::File::options()
            .read(true)
            .write(true)
            .open(&name)
            .expect("cannot open the pseudo-terminal");
        // its lock would keep bacnetio out
        drop(other_end);
        let (reader, writer) = tokio::io::split(device_end);
        let port = Port {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        };
        let config = MstpConfig {
            this_station: station,
            baud_rate: SPEED,
            ..MstpConfig::default()
        };
        (MstpTransport::new(port, config), name, kept_open)
    }
}

#[cfg(unix)]
#[tokio::main]
async fn main() {
    use bacnet_objects::analog::{AnalogInputObject, AnalogOutputObject};
    use bacnet_objects::device_identity::{DeviceIdentity, build_database_with_extra};
    use bacnet_objects::traits::BACnetObject;
    use bacnet_server::server::BACnetServer;
    use bacnet_types::enums::EngineeringUnits;

    let station: u8 = std::env::args()
        .nth(1)
        .map(|text| text.parse().expect("the station is a number from 0 to 127"))
        .unwrap_or(5);
    let instance = 2000 + u32::from(station);
    let name = format!("MSTP-{station:02}");
    let mut temperature =
        AnalogInputObject::new(1, "Room temperature", EngineeringUnits::DEGREES_CELSIUS).unwrap();
    temperature.set_present_value(21.5);
    let mut damper = AnalogOutputObject::new(1, "Damper", EngineeringUnits::PERCENT).unwrap();
    damper.set_relinquish_default(0.0).unwrap();
    let objects: Vec<Box<dyn BACnetObject>> = vec![Box::new(temperature), Box::new(damper)];
    // an MS/TP frame carries at most 480 octets of APDU (Clause 9.3)
    let identity = DeviceIdentity::new(instance, 42)
        .unwrap()
        .with_max_apdu(480)
        .unwrap()
        .with_name(&name);
    let database = build_database_with_extra(&identity, objects).unwrap();
    let (transport, terminal, other_end) = pty::open(station);
    tokio::spawn(pty::keep_openable(other_end));
    let _server = BACnetServer::generic_builder()
        .transport(transport)
        .device(identity)
        .database(database)
        .build()
        .await
        .expect("cannot start the simulated device");
    println!(
        "Simulated MS/TP device {instance} \"{name}\", station {station}, on {terminal} ({} baud). Ctrl+C to stop.",
        pty::SPEED
    );
    std::future::pending::<()>().await;
}

#[cfg(not(unix))]
fn main() {
    eprintln!("simulated_mstp needs a pseudo-terminal: Linux only");
}

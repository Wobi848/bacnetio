//! MS/TP over a serial port (docs/concept-mstp.md): the ports to choose from, and listening to a
//! trunk without joining it, to find its speed, its stations and a free address for bacnetio.

use std::time::Duration;

use anyhow::Context;
use bacnet_transport::mstp::monitor::{MonitorConfig, MonitorRead, MonitorStats, MstpMonitor};
use bacnet_transport::mstp_serial::{ReceiveOnlyPort, SerialConfig};
use serde::Serialize;

/// The speeds MS/TP trunks run at, the most common first (Clause 9.2.1 allows 9600 to 115200).
pub const SPEEDS: [u32; 5] = [38_400, 76_800, 19_200, 9_600, 115_200];
/// The highest master address on a trunk (Clause 9.5.2).
pub const MAX_MASTER: u8 = 127;
/// How long to listen at one speed: a token ring passes the token many times a second, and a
/// lone master still polls for others at least every few hundred milliseconds.
pub const LISTEN_PER_SPEED: Duration = Duration::from_secs(2);
/// Valid frames needed before a speed counts as the trunk's: one could be chance.
const FRAMES_FOR_A_SPEED: u64 = 3;

/// A serial port to choose, as the system names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SerialPortChoice {
    /// `COM3`, `/dev/ttyACM0`.
    pub name: String,
    /// What the adapter calls itself (`USB Single Serial`), when it says.
    pub description: Option<String>,
    pub usb: bool,
}

/// The serial ports of this computer, USB adapters first.
pub fn serial_ports() -> anyhow::Result<Vec<SerialPortChoice>> {
    let ports = tokio_serial::available_ports().context("cannot list the serial ports")?;
    let mut choices: Vec<SerialPortChoice> = ports
        .into_iter()
        .map(|port| match port.port_type {
            tokio_serial::SerialPortType::UsbPort(usb) => SerialPortChoice {
                name: port.port_name,
                description: usb.product.or(usb.manufacturer),
                usb: true,
            },
            _ => SerialPortChoice {
                name: port.port_name,
                description: None,
                usb: false,
            },
        })
        .collect();
    sort_ports(&mut choices);
    Ok(choices)
}

fn sort_ports(choices: &mut [SerialPortChoice]) {
    choices.sort_by(|a, b| b.usb.cmp(&a.usb).then_with(|| a.name.cmp(&b.name)));
}

/// What listening at one speed heard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Trial {
    pub baud: u32,
    /// Frames with a valid header and data.
    pub frames: u64,
    /// Frames the decoder refused: a broken CRC, a frame cut off, octets that make no frame.
    pub damaged: u64,
}

/// What bacnetio heard on a trunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Heard {
    /// The trunk's speed, or none when no speed gave clean frames.
    pub baud: Option<u32>,
    /// Every speed tried, in order.
    pub trials: Vec<Trial>,
    /// Stations that sent a frame at the trunk's speed.
    pub stations: Vec<u8>,
    /// The highest master address the stations poll for, as far as their polls show.
    pub max_master: Option<u8>,
    /// Silences long enough to mean the token was lost.
    pub lost_tokens: u64,
    /// The address proposed for bacnetio: the highest free one up to Max_Master.
    pub proposed_station: Option<u8>,
}

/// The trunk's speed: the speed tried with the most valid frames, with more valid frames than
/// damaged ones. A wrong speed yields garbage, rarely a frame whose CRC holds.
pub fn trunk_speed(trials: &[Trial]) -> Option<u32> {
    trials
        .iter()
        .filter(|trial| trial.frames >= FRAMES_FOR_A_SPEED && trial.frames > trial.damaged)
        .max_by_key(|trial| trial.frames)
        .map(|trial| trial.baud)
}

/// The highest address up to `max_master` that no station uses. Devices are usually numbered
/// from below, so the top collides least.
pub fn proposed_station(stations: &[u8], max_master: Option<u8>) -> Option<u8> {
    let highest = max_master.unwrap_or(MAX_MASTER).min(MAX_MASTER);
    (0..=highest)
        .rev()
        .find(|address| !stations.contains(address))
}

/// Listens at each speed in turn until one gives clean frames, opening the line anew for each.
/// `open` hands out a receive-only reader at a speed.
pub async fn listen<R, F>(mut open: F, speeds: &[u32], per_speed: Duration) -> anyhow::Result<Heard>
where
    R: MonitorRead + Send + 'static,
    F: FnMut(u32) -> anyhow::Result<R>,
{
    let mut trials = Vec::new();
    let mut found = None;
    for &baud in speeds {
        let stats = listen_at(open(baud)?, baud, per_speed).await?;
        trials.push(trial(baud, &stats));
        if trunk_speed(&trials) == Some(baud) {
            found = Some(stats);
            break;
        }
    }
    Ok(heard(trials, found.as_ref()))
}

async fn listen_at<R>(reader: R, baud: u32, duration: Duration) -> anyhow::Result<MonitorStats>
where
    R: MonitorRead + Send + 'static,
{
    let mut monitor = MstpMonitor::start(reader, MonitorConfig::new(baud))
        .await
        .with_context(|| format!("cannot listen at {baud} baud"))?;
    tokio::time::sleep(duration).await;
    monitor
        .stop()
        .await
        .with_context(|| format!("listening at {baud} baud failed"))
}

fn trial(baud: u32, stats: &MonitorStats) -> Trial {
    let errors = &stats.errors;
    Trial {
        baud,
        frames: stats.frames.total(),
        damaged: errors.header_crc + errors.data_crc + errors.malformed + errors.frame_aborts,
    }
}

fn heard(trials: Vec<Trial>, found: Option<&MonitorStats>) -> Heard {
    let Some(stats) = found else {
        return Heard {
            baud: None,
            trials,
            stations: Vec::new(),
            max_master: None,
            lost_tokens: 0,
            proposed_station: None,
        };
    };
    let stations: Vec<u8> = stats.present_stations().collect();
    let max_master = stats
        .stations
        .values()
        .filter_map(|station| station.max_master)
        .map(|max_master| max_master.value())
        .max();
    Heard {
        baud: trials.last().map(|trial| trial.baud),
        proposed_station: proposed_station(&stations, max_master),
        trials,
        stations,
        max_master,
        lost_tokens: stats.lost_tokens,
    }
}

/// Opens a serial port to listen only: nothing is sent.
pub fn open_receive_only(port: &str, baud: u32) -> anyhow::Result<ReceiveOnlyPort> {
    let config = SerialConfig {
        port_name: port.to_string(),
        baud_rate: baud,
    };
    ReceiveOnlyPort::open(&config, None).map_err(|error| {
        let error: anyhow::Error = error.into();
        if is_permission_denied(&error) {
            error.context(format!(
                "no permission to open {port}: add yourself to the group dialout \
                 (sudo usermod -aG dialout $USER) and log in again"
            ))
        } else {
            error.context(format!("cannot open {port}"))
        }
    })
}

fn is_permission_denied(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::PermissionDenied)
            || cause.to_string().contains("Permission denied")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(baud: u32, frames: u64, damaged: u64) -> Trial {
        Trial {
            baud,
            frames,
            damaged,
        }
    }

    #[test]
    fn the_speed_with_clean_frames_is_the_trunks() {
        let trials = [
            trial(38_400, 0, 12),
            trial(76_800, 40, 1),
            trial(19_200, 2, 0),
        ];
        assert_eq!(trunk_speed(&trials), Some(76_800));
    }

    #[test]
    fn mostly_damaged_frames_or_too_few_find_no_speed() {
        assert_eq!(trunk_speed(&[trial(38_400, 5, 9)]), None);
        assert_eq!(trunk_speed(&[trial(38_400, 2, 0)]), None);
        assert_eq!(trunk_speed(&[]), None);
    }

    #[test]
    fn the_highest_free_address_up_to_max_master_is_proposed() {
        assert_eq!(proposed_station(&[5, 6, 7], None), Some(127));
        assert_eq!(proposed_station(&[5, 63], Some(63)), Some(62));
        assert_eq!(proposed_station(&[], Some(200)), Some(127));
        let all: Vec<u8> = (0..=3).collect();
        assert_eq!(proposed_station(&all, Some(3)), None);
    }

    #[test]
    fn usb_adapters_come_first() {
        let port = |name: &str, usb| SerialPortChoice {
            name: name.to_string(),
            description: None,
            usb,
        };
        let mut ports = vec![
            port("/dev/ttyS0", false),
            port("/dev/ttyUSB1", true),
            port("/dev/ttyACM0", true),
        ];
        sort_ports(&mut ports);
        let names: Vec<&str> = ports.iter().map(|port| port.name.as_str()).collect();
        assert_eq!(names, ["/dev/ttyACM0", "/dev/ttyUSB1", "/dev/ttyS0"]);
    }
}

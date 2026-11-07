//! What a scan result means, in plain words (concept section 8).

use std::fmt;
use std::net::SocketAddrV4;

use serde::Serialize;

use crate::connection::BACNET_PORT;

use crate::interfaces::{Adapter, AdapterKind, MIN_SWEEP_PREFIX_LEN, Subnet};
use crate::scan::ScanReport;

/// How the subnet was asked address by address.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sweep {
    Done,
    Disabled,
    SubnetTooLarge { prefix_len: u8 },
}

/// Whether and which addresses to ask one by one: those of `subnet`, such as a plant behind a VPN,
/// or else of the adapter's own subnet.
pub fn plan_sweep(
    adapter: &Adapter,
    subnet: Option<Subnet>,
    enabled: bool,
    port: u16,
) -> (Sweep, Vec<SocketAddrV4>) {
    let subnet = subnet.unwrap_or_else(|| adapter.subnet());
    match (enabled, subnet.hosts(Some(adapter.ip))) {
        (false, _) => (Sweep::Disabled, Vec::new()),
        (true, None) => (
            Sweep::SubnetTooLarge {
                prefix_len: subnet.prefix_len,
            },
            Vec::new(),
        ),
        (true, Some(targets)) => (
            Sweep::Done,
            targets
                .into_iter()
                .map(|ip| SocketAddrV4::new(ip, port))
                .collect(),
        ),
    }
}

pub struct Context {
    pub sweep: Sweep,
    /// The adapter has a self-assigned address (169.254.x.x): no DHCP and no fixed IP.
    pub link_local: bool,
    /// Why port 47808 could not be used.
    pub port_unavailable: Option<String>,
    /// The adapter is a VPN or another virtual adapter, which seldom passes broadcasts on.
    pub virtual_adapter: bool,
}

impl Context {
    pub fn new(sweep: Sweep, adapter: &Adapter, port_unavailable: Option<String>) -> Self {
        Self {
            sweep,
            link_local: adapter.ip.is_link_local(),
            port_unavailable,
            virtual_adapter: adapter.kind() == AdapterKind::Virtual,
        }
    }
}

/// A finding about the scan. Shown in words through `Display`; the UI translates it by its code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Note {
    LinkLocal,
    PortInUse {
        reason: String,
    },
    SubnetTooLarge {
        prefix_len: u8,
        max_prefix_len: u8,
    },
    NoAnswer,
    NoAnswerSweepDisabled,
    NoAnswerSubnetTooLarge,
    NoneIdentified,
    BroadcastBlocked {
        direct: usize,
    },
    BroadcastNotPassed {
        direct: usize,
    },
    SomeOnlyDirect {
        direct: usize,
        total: usize,
    },
    DuplicateId {
        instance: u32,
        addresses: Vec<String>,
    },
    Refused {
        addresses: Vec<String>,
    },
    SilentNetworks {
        networks: Vec<u16>,
    },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Note::LinkLocal => write!(
                f,
                "The adapter has a self-assigned address (169.254.x.x): the network has no DHCP. \
                 Give the adapter a fixed IP in the plant's subnet."
            ),
            Note::PortInUse { reason } => write!(
                f,
                "The BACnet port is in use ({reason}), so another port was used. Devices that \
                 answer by broadcast cannot be heard. Close other BACnet programs for complete \
                 results."
            ),
            Note::SubnetTooLarge {
                prefix_len,
                max_prefix_len,
            } => write!(
                f,
                "The subnet (/{prefix_len}) is too large to ask every address; only Who-Is was \
                 used. Asking each address works up to /{max_prefix_len}."
            ),
            Note::NoAnswer => write!(
                f,
                "No device answered, neither to Who-Is nor when asked directly. Check the \
                 adapter, cable, VLAN, and whether the plant uses port {BACNET_PORT}."
            ),
            Note::NoAnswerSweepDisabled => write!(
                f,
                "No device answered the Who-Is broadcast. Run without --no-sweep to ask every \
                 address directly; that also works behind a firewall."
            ),
            Note::NoAnswerSubnetTooLarge => write!(
                f,
                "No device answered the Who-Is broadcast. Check the adapter, cable and VLAN."
            ),
            Note::NoneIdentified => write!(
                f,
                "No device could be identified, but some addresses answered (see below)."
            ),
            Note::BroadcastBlocked { direct } => write!(
                f,
                "No device answered the Who-Is broadcast, but {direct} answered when asked \
                 directly. Broadcast answers are most likely blocked by the firewall, which is \
                 common without administrator rights. bacnetio works around it."
            ),
            Note::BroadcastNotPassed { direct } => write!(
                f,
                "No device answered the Who-Is broadcast, but {direct} answered when asked \
                 directly. A VPN seldom passes broadcasts on, so bacnetio asks every address."
            ),
            Note::SomeOnlyDirect { direct, total } => write!(
                f,
                "{direct} of {total} devices answered only when asked directly: their broadcast \
                 answers do not arrive (firewall)."
            ),
            Note::DuplicateId {
                instance,
                addresses,
            } => write!(
                f,
                "Device ID {instance} answers from {}: two devices share one ID. Values may be \
                 read from the wrong device until this is fixed.",
                addresses.join(" and ")
            ),
            Note::Refused { addresses } => write!(
                f,
                "{} address(es) answered but refused the direct question ({}): there is BACnet, \
                 but the device(s) only answer when asked by device ID.",
                addresses.len(),
                addresses.join(", ")
            ),
            Note::SilentNetworks { networks } => write!(
                f,
                "Routers reach network(s) {} where no device answered. Devices behind a router \
                 (often MS/TP) may be switched off, wired wrongly, or have a wrong MAC address or \
                 baud rate.",
                networks
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

pub fn notes(report: &ScanReport, context: &Context) -> Vec<Note> {
    let mut notes = Vec::new();
    if context.link_local {
        notes.push(Note::LinkLocal);
    }
    if let Some(reason) = &context.port_unavailable {
        notes.push(port_note(reason));
    }
    if let Sweep::SubnetTooLarge { prefix_len } = context.sweep {
        notes.push(Note::SubnetTooLarge {
            prefix_len,
            max_prefix_len: MIN_SWEEP_PREFIX_LEN,
        });
    }

    let answered_who_is = report.devices.iter().filter(|d| d.answered_who_is).count();
    let direct = report.devices.len() - answered_who_is;
    if report.devices.is_empty() {
        notes.push(nothing_found(context.sweep, report.refused.is_empty()));
    } else if answered_who_is == 0 && context.virtual_adapter {
        notes.push(Note::BroadcastNotPassed { direct });
    } else if answered_who_is == 0 {
        notes.push(Note::BroadcastBlocked { direct });
    } else if direct > 0 {
        notes.push(Note::SomeOnlyDirect {
            direct,
            total: report.devices.len(),
        });
    }

    for (instance, addresses) in &report.duplicates {
        notes.push(Note::DuplicateId {
            instance: *instance,
            addresses: addresses.iter().map(|a| a.shown()).collect(),
        });
    }
    if !report.refused.is_empty() {
        notes.push(Note::Refused {
            addresses: report
                .refused
                .iter()
                .map(|(a, _)| a.ip().to_string())
                .collect(),
        });
    }
    let silent = silent_networks(report);
    if !silent.is_empty() {
        notes.push(Note::SilentNetworks { networks: silent });
    }
    notes
}

/// Networks a router reaches but no device was found on.
fn silent_networks(report: &ScanReport) -> Vec<u16> {
    let mut networks: Vec<u16> = report
        .routers
        .iter()
        .flat_map(|router| router.networks.iter().copied())
        .filter(|network| {
            !report.devices.iter().any(|device| {
                device
                    .remote
                    .as_ref()
                    .is_some_and(|r| r.network == *network)
            })
        })
        .collect();
    networks.sort_unstable();
    networks.dedup();
    networks
}

/// The BACnet port was taken by another program.
pub fn port_note(reason: &str) -> Note {
    Note::PortInUse {
        reason: reason.to_string(),
    }
}

fn nothing_found(sweep: Sweep, nothing_refused: bool) -> Note {
    if !nothing_refused {
        return Note::NoneIdentified;
    }
    match sweep {
        Sweep::Done => Note::NoAnswer,
        Sweep::Disabled => Note::NoAnswerSweepDisabled,
        Sweep::SubnetTooLarge { .. } => Note::NoAnswerSubnetTooLarge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{Details, Device, RemoteStation, Router};
    use std::net::{Ipv4Addr, SocketAddrV4};

    fn device(instance: u32, answered_who_is: bool) -> Device {
        Device {
            instance,
            address: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, instance as u8), BACNET_PORT).into(),
            remote: None,
            answered_who_is,
            details: Details::default(),
            objects: Default::default(),
        }
    }

    fn context() -> Context {
        Context {
            sweep: Sweep::Done,
            link_local: false,
            port_unavailable: None,
            virtual_adapter: false,
        }
    }

    #[test]
    fn firewall_suspected_when_only_direct_answers() {
        let report = ScanReport {
            devices: vec![device(1, false), device(2, false)],
            ..ScanReport::default()
        };
        let notes = notes(&report, &context());
        assert_eq!(notes.len(), 1);
        assert!(matches!(notes[0], Note::BroadcastBlocked { direct: 2 }));
        assert!(notes[0].to_string().contains("firewall"));
    }

    #[test]
    fn behind_a_vpn_the_vpn_is_named_not_the_firewall() {
        let report = ScanReport {
            devices: vec![device(1, false)],
            ..ScanReport::default()
        };
        let context = Context {
            virtual_adapter: true,
            ..context()
        };
        let notes = notes(&report, &context);
        assert!(matches!(notes[0], Note::BroadcastNotPassed { direct: 1 }));
        assert!(notes[0].to_string().contains("VPN"));
    }

    #[test]
    fn partial_broadcast_answers_are_named() {
        let report = ScanReport {
            devices: vec![device(1, true), device(2, false)],
            ..ScanReport::default()
        };
        assert!(
            notes(&report, &context())[0]
                .to_string()
                .starts_with("1 of 2 devices")
        );
    }

    #[test]
    fn all_found_by_who_is_needs_no_note() {
        let report = ScanReport {
            devices: vec![device(1, true)],
            ..ScanReport::default()
        };
        assert!(notes(&report, &context()).is_empty());
    }

    #[test]
    fn networks_without_devices_are_named() {
        let mut remote = device(1, true);
        remote.remote = Some(RemoteStation {
            network: 5,
            mac: vec![3],
        });
        let router = |networks: Vec<u16>| Router {
            address: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 9), BACNET_PORT).into(),
            networks,
        };
        let report = ScanReport {
            devices: vec![remote],
            routers: vec![router(vec![5, 7]), router(vec![7, 9])],
            ..ScanReport::default()
        };
        assert_eq!(
            notes(&report, &context()),
            vec![Note::SilentNetworks {
                networks: vec![7, 9]
            }]
        );
    }

    #[test]
    fn nothing_found_suggests_the_sweep_when_disabled() {
        let context = Context {
            sweep: Sweep::Disabled,
            ..context()
        };
        assert!(
            notes(&ScanReport::default(), &context)[0]
                .to_string()
                .contains("--no-sweep")
        );
    }

    #[test]
    fn port_in_use_and_duplicates_are_reported() {
        let mut report = ScanReport {
            devices: vec![device(1, true), device(1, false)],
            ..ScanReport::default()
        };
        let context = Context {
            port_unavailable: Some("address in use".into()),
            ..context()
        };
        report.duplicates.insert(
            1,
            report.devices.iter().map(|device| device.address).collect(),
        );
        let notes = notes(&report, &context);
        assert!(
            notes
                .iter()
                .any(|note| note.to_string().contains("The BACnet port is in use"))
        );
        assert!(
            notes
                .iter()
                .any(|note| note.to_string().contains("Device ID 1 answers from"))
        );
    }
}

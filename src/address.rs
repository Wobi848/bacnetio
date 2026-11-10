//! Where a device answers on the local link: an IP address and port on BACnet/IP, a station
//! address on an MS/TP trunk (docs/concept-mstp.md), or a VMAC on a BACnet/SC network
//! (docs/concept-sc.md). Written as text in files and the API: `10.0.0.20:47808` as before,
//! `mstp:5` for a station, `sc:20:00:00:00:00:01` for a VMAC.

use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MSTP_PREFIX: &str = "mstp:";
/// How the interface shows a station, read back where a site file holds it so.
const MSTP_SHOWN: &str = "MS/TP ";
const SC_PREFIX: &str = "sc:";
const SC_SHOWN: &str = "SC ";

/// The kind of local link bacnetio is on. A MAC alone does not say it: a VMAC on BACnet/SC has six
/// octets, as an IP address with its port has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Ip,
    Mstp,
    Sc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Address {
    Ip(SocketAddrV4),
    Mstp(u8),
    Sc([u8; 6]),
}

impl Address {
    /// The MAC address BACnet uses for it: six octets of IP and port, the station's one, or the
    /// VMAC.
    pub fn mac(self) -> Vec<u8> {
        match self {
            Address::Ip(address) => {
                let [a, b, c, d] = address.ip().octets();
                let [high, low] = address.port().to_be_bytes();
                vec![a, b, c, d, high, low]
            }
            Address::Mstp(station) => vec![station],
            Address::Sc(vmac) => vmac.to_vec(),
        }
    }

    /// The address a MAC stands for on a link of that kind.
    pub fn from_mac(link: Link, mac: &[u8]) -> Option<Address> {
        match (link, mac) {
            (Link::Ip, &[a, b, c, d, high, low]) => Some(Address::Ip(SocketAddrV4::new(
                Ipv4Addr::new(a, b, c, d),
                u16::from_be_bytes([high, low]),
            ))),
            (Link::Mstp, &[station]) => Some(Address::Mstp(station)),
            (Link::Sc, &[a, b, c, d, e, f]) => Some(Address::Sc([a, b, c, d, e, f])),
            _ => None,
        }
    }

    /// The kind of link the device answers on.
    pub fn link(self) -> Link {
        match self {
            Address::Ip(_) => Link::Ip,
            Address::Mstp(_) => Link::Mstp,
            Address::Sc(_) => Link::Sc,
        }
    }

    /// For people: the IP alone on the standard port, else IP:PORT; a station as `MS/TP 5`.
    pub fn shown(self) -> String {
        match self {
            Address::Ip(address) if address.port() == crate::connection::BACNET_PORT => {
                address.ip().to_string()
            }
            Address::Ip(address) => address.to_string(),
            Address::Mstp(station) => format!("{MSTP_SHOWN}{station}"),
            Address::Sc(vmac) => format!("{SC_SHOWN}{}", vmac_text(vmac)),
        }
    }

    /// The IP address, for what only BACnet/IP has (BBMD tables, the firewall diagnosis).
    pub fn ip(self) -> Option<SocketAddrV4> {
        match self {
            Address::Ip(address) => Some(address),
            Address::Mstp(_) | Address::Sc(_) => None,
        }
    }
}

/// Six octets in hex, separated by colons, as a VMAC is written.
fn vmac_text(vmac: [u8; 6]) -> String {
    vmac.map(|octet| format!("{octet:02x}")).join(":")
}

fn parse_vmac(text: &str) -> Option<[u8; 6]> {
    let mut vmac = [0; 6];
    let mut octets = text.split(':');
    for slot in &mut vmac {
        let octet = octets.next()?;
        if octet.len() != 2 {
            return None;
        }
        *slot = u8::from_str_radix(octet, 16).ok()?;
    }
    octets.next().is_none().then_some(vmac)
}

impl From<SocketAddrV4> for Address {
    fn from(address: SocketAddrV4) -> Self {
        Address::Ip(address)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Ip(address) => write!(f, "{address}"),
            Address::Mstp(station) => write!(f, "{MSTP_PREFIX}{station}"),
            Address::Sc(vmac) => write!(f, "{SC_PREFIX}{}", vmac_text(*vmac)),
        }
    }
}

impl FromStr for Address {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Some(station) = text
            .strip_prefix(MSTP_PREFIX)
            .or_else(|| text.strip_prefix(MSTP_SHOWN))
        {
            return station
                .parse()
                .map(Address::Mstp)
                .map_err(|_| format!("{text} is no MS/TP station"));
        }
        if let Some(vmac) = text
            .strip_prefix(SC_PREFIX)
            .or_else(|| text.strip_prefix(SC_SHOWN))
        {
            return parse_vmac(vmac)
                .map(Address::Sc)
                .ok_or_else(|| format!("{text} is no BACnet/SC VMAC"));
        }
        text.parse()
            .map(Address::Ip)
            .map_err(|_| format!("{text} is no device address"))
    }
}

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_round_trip() {
        let ip = Address::Ip("192.168.10.20:47808".parse().unwrap());
        assert_eq!(ip.mac(), [192, 168, 10, 20, 0xBA, 0xC0]);
        assert_eq!(Address::from_mac(Link::Ip, &ip.mac()), Some(ip));
        assert_eq!(Address::from_mac(Link::Mstp, &[5]), Some(Address::Mstp(5)));
        assert_eq!(Address::from_mac(Link::Ip, &[1, 2, 3]), None);
        // six octets on SC are a VMAC, not an IP address
        let vmac = [0x20, 0, 0, 0, 0, 1];
        assert_eq!(Address::from_mac(Link::Sc, &vmac), Some(Address::Sc(vmac)));
        assert_eq!(Address::Sc(vmac).mac(), vmac);
        assert_eq!(Address::from_mac(Link::Sc, &[5]), None);
    }

    #[test]
    fn a_vmac_written_with_a_prefix_and_shown_with_sc() {
        let device: Address = serde_json::from_str("\"sc:20:00:00:00:00:1f\"").unwrap();
        assert_eq!(device, Address::Sc([0x20, 0, 0, 0, 0, 0x1f]));
        assert_eq!(device.to_string(), "sc:20:00:00:00:00:1f");
        assert_eq!(device.shown(), "SC 20:00:00:00:00:1f");
        assert_eq!("SC 20:00:00:00:00:1F".parse::<Address>(), Ok(device));
        assert_eq!(device.link(), Link::Sc);
        assert_eq!(device.ip(), None);
        for wrong in [
            "sc:20:00:00:00:00",
            "sc:20:00:00:00:00:01:02",
            "sc:2:00:00:00:00:01",
            "sc:zz:00:00:00:00:01",
        ] {
            assert!(wrong.parse::<Address>().is_err(), "{wrong}");
        }
    }

    #[test]
    fn written_as_before_for_ip_and_with_a_prefix_for_mstp() {
        let ip: Address = "10.0.0.20:47808".parse().unwrap();
        assert_eq!(ip.to_string(), "10.0.0.20:47808");
        assert_eq!(serde_json::to_string(&ip).unwrap(), "\"10.0.0.20:47808\"");
        // a file written before MS/TP loads unchanged
        let old: Address = serde_json::from_str("\"10.0.0.20:47808\"").unwrap();
        assert_eq!(old, ip);
        let station: Address = serde_json::from_str("\"mstp:12\"").unwrap();
        assert_eq!(station, Address::Mstp(12));
        assert_eq!(station.to_string(), "mstp:12");
        assert!("mstp:300".parse::<Address>().is_err());
        // as the interface shows it, and as a site file keeps it
        assert_eq!(station.shown(), "MS/TP 12");
        assert_eq!("MS/TP 12".parse::<Address>(), Ok(station));
        assert_eq!(ip.shown(), "10.0.0.20");
    }
}

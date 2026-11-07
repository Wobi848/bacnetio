//! Where a device answers on the local link: an IP address and port on BACnet/IP, or a station
//! address on an MS/TP trunk (docs/concept-mstp.md). Written as text in files and the API:
//! `10.0.0.20:47808` as before, `mstp:5` for a station.

use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MSTP_PREFIX: &str = "mstp:";
/// How the interface shows a station, read back where a site file holds it so.
const MSTP_SHOWN: &str = "MS/TP ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Address {
    Ip(SocketAddrV4),
    Mstp(u8),
}

impl Address {
    /// The MAC address BACnet uses for it: six octets of IP and port, or the station's one.
    pub fn mac(self) -> Vec<u8> {
        match self {
            Address::Ip(address) => {
                let [a, b, c, d] = address.ip().octets();
                let [high, low] = address.port().to_be_bytes();
                vec![a, b, c, d, high, low]
            }
            Address::Mstp(station) => vec![station],
        }
    }

    /// The address a MAC stands for on the local link, by its length.
    pub fn from_mac(mac: &[u8]) -> Option<Address> {
        match *mac {
            [a, b, c, d, high, low] => Some(Address::Ip(SocketAddrV4::new(
                Ipv4Addr::new(a, b, c, d),
                u16::from_be_bytes([high, low]),
            ))),
            [station] => Some(Address::Mstp(station)),
            _ => None,
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
        }
    }

    /// The IP address, for what only BACnet/IP has (BBMD tables, the firewall diagnosis).
    pub fn ip(self) -> Option<SocketAddrV4> {
        match self {
            Address::Ip(address) => Some(address),
            Address::Mstp(_) => None,
        }
    }
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
        assert_eq!(Address::from_mac(&ip.mac()), Some(ip));
        assert_eq!(Address::from_mac(&[5]), Some(Address::Mstp(5)));
        assert_eq!(Address::from_mac(&[1, 2, 3]), None);
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

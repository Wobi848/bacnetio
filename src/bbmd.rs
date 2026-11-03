//! The tables a BBMD keeps: the Broadcast Distribution Table (which BBMDs it passes broadcasts
//! to) and the Foreign Device Table (which devices are registered with it). Read only: bacnetio
//! never writes a BBMD table (concept-explorer section 2b).

use std::fmt;
use std::net::{Ipv4Addr, SocketAddrV4};

use bacnet_transport::bip::BvlcExchangeError;
use serde::Serialize;

use crate::connection::{Client, mac};

/// One BBMD the asked BBMD distributes broadcasts to.
#[derive(Debug, Clone, PartialEq)]
pub struct Peer {
    pub address: SocketAddrV4,
    /// All ones means the broadcast goes to that BBMD itself (two-hop), else to its subnet.
    pub mask: Ipv4Addr,
}

/// One device registered with the BBMD as a foreign device.
#[derive(Debug, Clone, PartialEq)]
pub struct Foreign {
    pub address: SocketAddrV4,
    /// The lifetime it asked for, in seconds.
    pub ttl_s: u16,
    /// What the BBMD says is left of it, grace period included.
    pub seconds_left: u16,
}

/// Why a table could not be read. Shown in words through `Display`; the UI translates by code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum TableNote {
    /// The device answered with a refusal, as every node that is no BBMD does.
    NoBbmd,
    /// Nothing came back: no BBMD there, or the answer does not get through.
    Silent,
    /// Something else went wrong, such as an answer that could not be read.
    Failed { reason: String },
}

impl fmt::Display for TableNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableNote::NoBbmd => write!(
                f,
                "The device refused the table read: it is no BBMD, or it does not give its tables."
            ),
            TableNote::Silent => write!(
                f,
                "The device did not answer the table read. Only a BBMD answers it."
            ),
            TableNote::Failed { reason } => write!(f, "The table could not be read: {reason}"),
        }
    }
}

#[derive(Debug)]
pub struct Tables {
    pub broadcast: Result<Vec<Peer>, TableNote>,
    pub foreign: Result<Vec<Foreign>, TableNote>,
}

impl Tables {
    /// A device that gives its Broadcast Distribution Table is a BBMD.
    pub fn is_bbmd(&self) -> bool {
        self.broadcast.is_ok()
    }
}

/// Reads both tables of the device at `address`, one request each. A device that is no BBMD
/// refuses or stays silent, which is an answer, not a failure.
pub async fn read_tables(client: &Client, address: SocketAddrV4) -> Tables {
    let target = mac(address);
    let broadcast = match client.read_bdt(&target).await {
        Ok(entries) => Ok(entries
            .into_iter()
            .map(|entry| Peer {
                address: SocketAddrV4::new(Ipv4Addr::from(entry.ip), entry.port),
                mask: Ipv4Addr::from(entry.broadcast_mask),
            })
            .collect()),
        Err(error) => Err(note(&error)),
    };
    let foreign = match client.read_fdt(&target).await {
        Ok(entries) => Ok(entries
            .into_iter()
            .map(|entry| Foreign {
                address: SocketAddrV4::new(Ipv4Addr::from(entry.ip), entry.port),
                ttl_s: entry.ttl,
                seconds_left: entry.seconds_remaining,
            })
            .collect()),
        Err(error) => Err(note(&error)),
    };
    Tables { broadcast, foreign }
}

fn note(error: &BvlcExchangeError) -> TableNote {
    match error {
        BvlcExchangeError::Nak(_) => TableNote::NoBbmd,
        BvlcExchangeError::NoReply(_) => TableNote::Silent,
        other => TableNote::Failed {
            reason: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_transport::bvll::BvlcResultCode;

    #[test]
    fn a_refusal_means_no_bbmd_and_silence_means_nothing_there() {
        assert_eq!(
            note(&BvlcExchangeError::Nak(
                BvlcResultCode::READ_BROADCAST_DISTRIBUTION_TABLE_NAK
            )),
            TableNote::NoBbmd
        );
        assert_eq!(
            note(&BvlcExchangeError::NoReply(std::time::Duration::from_secs(
                3
            ))),
            TableNote::Silent
        );
        assert!(TableNote::NoBbmd.to_string().contains("no BBMD"));
    }
}

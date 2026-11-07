//! A capture file Wireshark opens (pcapng): BACnet/IP datagrams as IPv4 packets, MS/TP frames as
//! they are on the line (docs/concept-toolbox.md part 3).
//!
//! bacnetio sees the BVLL message of a datagram, not the packet around it, so the IPv4 and UDP
//! headers are built from the two addresses; the file's comment says so.

use std::net::SocketAddrV4;
use std::time::{SystemTime, UNIX_EPOCH};

/// One packet to write.
pub enum Packet<'a> {
    /// A BACnet/IP datagram's payload, the BVLL message.
    Udp {
        time: SystemTime,
        source: SocketAddrV4,
        destination: SocketAddrV4,
        payload: &'a [u8],
    },
    /// A whole MS/TP frame, preamble to CRC.
    Mstp { time: SystemTime, frame: &'a [u8] },
}

/// LINKTYPE_IPV4: a raw IPv4 packet.
const LINK_IPV4: u16 = 228;
/// LINKTYPE_BACNET_MS_TP: a frame from its preamble on.
const LINK_MSTP: u16 = 165;
/// The interfaces of the file, in this order: their number is their place.
const INTERFACES: [u16; 2] = [LINK_IPV4, LINK_MSTP];
const IPV4_INTERFACE: u32 = 0;
const MSTP_INTERFACE: u32 = 1;

const SECTION_HEADER: u32 = 0x0A0D_0D0A;
const INTERFACE_DESCRIPTION: u32 = 1;
const ENHANCED_PACKET: u32 = 6;
const BYTE_ORDER_MAGIC: u32 = 0x1A2B_3C4D;
const OPTION_COMMENT: u16 = 1;
const OPTION_END: u16 = 0;
const SNAPLEN: u32 = 65_535;
const UDP: u8 = 17;
const TTL: u8 = 64;

/// The packets as a pcapng file, with `comment` in its header.
pub fn pcapng(packets: &[Packet], comment: &str) -> Vec<u8> {
    let mut file = block(SECTION_HEADER, &section_body(comment));
    for link in INTERFACES {
        let mut body = Vec::new();
        body.extend(link.to_le_bytes());
        body.extend(0u16.to_le_bytes());
        body.extend(SNAPLEN.to_le_bytes());
        file.extend(block(INTERFACE_DESCRIPTION, &body));
    }
    for packet in packets {
        let (interface, time, data) = match packet {
            Packet::Udp {
                time,
                source,
                destination,
                payload,
            } => (
                IPV4_INTERFACE,
                time,
                ipv4_udp(*source, *destination, payload),
            ),
            Packet::Mstp { time, frame } => (MSTP_INTERFACE, time, frame.to_vec()),
        };
        file.extend(block(
            ENHANCED_PACKET,
            &packet_body(interface, *time, &data),
        ));
    }
    file
}

fn section_body(comment: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(BYTE_ORDER_MAGIC.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    // the length of the section is not given
    body.extend((-1i64).to_le_bytes());
    body.extend(option(OPTION_COMMENT, comment.as_bytes()));
    body.extend(option(OPTION_END, &[]));
    body
}

fn packet_body(interface: u32, time: SystemTime, data: &[u8]) -> Vec<u8> {
    // microseconds since 1970, the default resolution of an interface
    let micros = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_micros() as u64);
    let length = data.len() as u32;
    let mut body = Vec::new();
    body.extend(interface.to_le_bytes());
    body.extend(((micros >> 32) as u32).to_le_bytes());
    body.extend((micros as u32).to_le_bytes());
    body.extend(length.to_le_bytes());
    body.extend(length.to_le_bytes());
    body.extend(data);
    pad(&mut body);
    body
}

fn option(code: u16, value: &[u8]) -> Vec<u8> {
    let mut option = Vec::new();
    option.extend(code.to_le_bytes());
    option.extend((value.len() as u16).to_le_bytes());
    option.extend(value);
    pad(&mut option);
    option
}

/// A block: its type, its length before and after the body.
fn block(kind: u32, body: &[u8]) -> Vec<u8> {
    let length = (body.len() + 12) as u32;
    let mut block = Vec::with_capacity(length as usize);
    block.extend(kind.to_le_bytes());
    block.extend(length.to_le_bytes());
    block.extend(body);
    block.extend(length.to_le_bytes());
    block
}

fn pad(bytes: &mut Vec<u8>) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
}

/// An IPv4 packet with a UDP datagram, without a UDP checksum (allowed on IPv4).
fn ipv4_udp(source: SocketAddrV4, destination: SocketAddrV4, payload: &[u8]) -> Vec<u8> {
    let udp_length = (8 + payload.len()) as u16;
    let total = 20 + udp_length;
    let mut packet = vec![0x45, 0];
    packet.extend(total.to_be_bytes());
    // identification, flags and fragment offset
    packet.extend([0, 0, 0, 0]);
    packet.extend([TTL, UDP, 0, 0]);
    packet.extend(source.ip().octets());
    packet.extend(destination.ip().octets());
    let checksum = header_checksum(&packet);
    packet[10..12].copy_from_slice(&checksum.to_be_bytes());
    packet.extend(source.port().to_be_bytes());
    packet.extend(destination.port().to_be_bytes());
    packet.extend(udp_length.to_be_bytes());
    packet.extend([0, 0]);
    packet.extend(payload);
    packet
}

fn header_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = header
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], pair[1]])))
        .sum();
    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::Duration;

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    /// The blocks of a file, as (type, body) pairs, checking that each length is told twice.
    fn blocks(file: &[u8]) -> Vec<(u32, &[u8])> {
        let mut found = Vec::new();
        let mut at = 0;
        while at < file.len() {
            let length = u32_at(file, at + 4) as usize;
            assert_eq!(u32_at(file, at + length - 4) as usize, length);
            assert_eq!(length % 4, 0);
            found.push((u32_at(file, at), &file[at + 8..at + length - 4]));
            at += length;
        }
        found
    }

    #[test]
    fn a_datagram_becomes_an_ipv4_packet_with_its_addresses() {
        let time = UNIX_EPOCH + Duration::from_micros(1_700_000_000_123_456);
        let payload = [0x81, 0x0B, 0x00, 0x08, 0x01, 0x20, 0xFF, 0xFF];
        let file = pcapng(
            &[Packet::Udp {
                time,
                source: SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 10), 47808),
                destination: SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 255), 47808),
                payload: &payload,
            }],
            "headers built by bacnetio",
        );
        let found = blocks(&file);
        assert_eq!(
            found.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
            [
                SECTION_HEADER,
                INTERFACE_DESCRIPTION,
                INTERFACE_DESCRIPTION,
                ENHANCED_PACKET
            ]
        );
        assert_eq!(u32_at(found[0].1, 0), BYTE_ORDER_MAGIC);
        let packet = found[3].1;
        assert_eq!(u32_at(packet, 0), IPV4_INTERFACE);
        let micros = (u64::from(u32_at(packet, 4)) << 32) | u64::from(u32_at(packet, 8));
        assert_eq!(micros, 1_700_000_000_123_456);
        let length = u32_at(packet, 12) as usize;
        assert_eq!(length, 20 + 8 + payload.len());
        let ip = &packet[20..20 + length];
        assert_eq!(ip[0], 0x45);
        assert_eq!(ip[9], UDP);
        assert_eq!(&ip[12..16], &[192, 168, 1, 10]);
        assert_eq!(&ip[16..20], &[192, 168, 1, 255]);
        assert_eq!(
            header_checksum(&ip[..20]),
            0,
            "a correct checksum sums to nothing"
        );
        assert_eq!(u16::from_be_bytes([ip[22], ip[23]]), 47808);
        assert_eq!(&ip[28..], &payload);
    }

    #[test]
    fn an_mstp_frame_is_written_as_it_was_on_the_line() {
        let frame = [0x55, 0xFF, 0x00, 0x05, 0x7F, 0x00, 0x00, 0x8D];
        let file = pcapng(
            &[Packet::Mstp {
                time: UNIX_EPOCH,
                frame: &frame,
            }],
            "",
        );
        let found = blocks(&file);
        let interface = found[2].1;
        assert_eq!(u16::from_le_bytes([interface[0], interface[1]]), LINK_MSTP);
        let packet = found[3].1;
        assert_eq!(u32_at(packet, 0), MSTP_INTERFACE);
        assert_eq!(&packet[20..20 + frame.len()], &frame);
    }
}

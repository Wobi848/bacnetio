//! bacnetio's own BACnet traffic, captured on request to be saved for Wireshark
//! (docs/concept-toolbox.md part 3). On a technician's laptop without administrator rights Wireshark
//! cannot capture; bacnetio sees its traffic anyway, through the library's frame tap. While no
//! capture runs, nothing is subscribed and nothing is copied.

use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::{Arc, Mutex};

use bacnet_transport::tap::{TapDirection, TapPeer, TappedFrame};
use serde::Serialize;

use crate::connection::Client;
use crate::pcap::{self, Packet};

/// Frames kept at most: past that the oldest go. Some 100 MB at the largest frames.
pub const MOST_FRAMES: usize = 100_000;
/// Frames the tap may run ahead of the capture before it drops some.
const TAP_QUEUE: usize = 4096;
/// What the file says about itself, since the IP headers are not the ones on the wire.
const FILE_COMMENT: &str = "Captured by bacnetio: its own BACnet traffic. On BACnet/IP the IPv4 and \
UDP headers are built from the addresses; the BVLL messages are as sent and received. On MS/TP \
every frame on the line, as heard.";

struct Captured {
    frame: TappedFrame,
    /// This end of a BACnet/IP datagram; none on MS/TP.
    local: Option<SocketAddrV4>,
}

#[derive(Default)]
struct Inner {
    running: bool,
    frames: VecDeque<Captured>,
    /// Frames that went because the capture was full.
    overflowed: u64,
    /// The tasks that take frames from each connection's tap.
    followers: Vec<tokio::task::AbortHandle>,
}

#[derive(Default)]
pub struct Capture {
    inner: Mutex<Inner>,
}

#[derive(Serialize)]
pub struct CaptureState {
    pub running: bool,
    pub frames: usize,
    pub overflowed: u64,
}

impl Capture {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("capture lock")
    }

    pub fn state(&self) -> CaptureState {
        let inner = self.lock();
        CaptureState {
            running: inner.running,
            frames: inner.frames.len(),
            overflowed: inner.overflowed,
        }
    }

    /// Starts capturing, from the connection open now if there is one.
    pub fn start(self: &Arc<Self>, client: Option<&Client>) {
        self.lock().running = true;
        if let Some(client) = client {
            self.follow(client);
        }
    }

    /// Takes the frames of a connection while a capture runs: one opened now, or a new one.
    pub fn follow(self: &Arc<Self>, client: &Client) {
        let mut inner = self.lock();
        if !inner.running {
            return;
        }
        let Some(tap) = client.transport().frame_tap() else {
            return;
        };
        let local = bip_endpoint(client.local_mac());
        let mut frames = tap.subscribe_with_capacity(TAP_QUEUE);
        let capture = Arc::clone(self);
        let task = tokio::spawn(async move {
            while let Some(frame) = frames.recv().await {
                capture.keep(Captured { frame, local });
            }
        });
        inner.followers.push(task.abort_handle());
    }

    fn keep(&self, captured: Captured) {
        let mut inner = self.lock();
        if !inner.running {
            return;
        }
        if inner.frames.len() >= MOST_FRAMES {
            inner.frames.pop_front();
            inner.overflowed += 1;
        }
        inner.frames.push_back(captured);
    }

    /// Stops capturing; what was captured stays until cleared.
    pub fn stop(&self) {
        let mut inner = self.lock();
        inner.running = false;
        for follower in inner.followers.drain(..) {
            follower.abort();
        }
    }

    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.frames.clear();
        inner.overflowed = 0;
    }

    /// What was captured, as a pcapng file.
    pub fn pcapng(&self) -> Vec<u8> {
        let inner = self.lock();
        let packets: Vec<Packet> = inner.frames.iter().filter_map(packet).collect();
        pcap::pcapng(&packets, FILE_COMMENT)
    }
}

/// A frame as a packet of the file: a datagram between this end and the peer, or an MS/TP frame.
fn packet(captured: &Captured) -> Option<Packet<'_>> {
    let frame = &captured.frame;
    match frame.peer {
        TapPeer::Udp(std::net::SocketAddr::V4(peer)) => {
            let local = captured
                .local
                .unwrap_or(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
            let (source, destination) = match frame.direction {
                TapDirection::Outbound => (local, peer),
                _ => (peer, local),
            };
            Some(Packet::Udp {
                time: frame.timestamp,
                source,
                destination,
                payload: &frame.bytes,
            })
        }
        TapPeer::Mstp(_) => Some(Packet::Mstp {
            time: frame.timestamp,
            frame: &frame.bytes,
        }),
        // IPv6 and BACnet/SC: bacnetio opens neither
        _ => None,
    }
}

/// A BACnet/IP MAC is the address and the port: six octets.
fn bip_endpoint(mac: &[u8]) -> Option<SocketAddrV4> {
    let [a, b, c, d, high, low] = mac.try_into().ok()?;
    Some(SocketAddrV4::new(
        Ipv4Addr::new(a, b, c, d),
        u16::from_be_bytes([high, low]),
    ))
}

//! The devices of a running scan, merged from Who-Is answers, direct answers and details as they
//! come, so that the list can be shown while the scan runs.

use std::collections::BTreeSet;
use std::net::SocketAddrV4;

use bacnet_client::discovery::DiscoveredDevice;

use super::{Details, Device, ObjectList, Phase, Progress, RemoteStation};
use crate::address::Address;

#[derive(Default)]
pub(super) struct Live {
    pub devices: Vec<Device>,
    pub refused: Vec<(SocketAddrV4, String)>,
    pub asked: usize,
    pub total: usize,
    /// Devices whose details are being read or were read.
    detailed: BTreeSet<(u32, Address)>,
    /// Devices found by asking directly and made known to the client by us; the client then
    /// lists them like answers to Who-Is.
    added: BTreeSet<(u32, Address)>,
}

impl Live {
    pub fn new(total: usize) -> Self {
        Live {
            total,
            ..Live::default()
        }
    }

    /// Devices that answered Who-Is; a device asked directly before is marked as answering too.
    pub fn merge(&mut self, found: Vec<DiscoveredDevice>) {
        for found in found {
            let Some(address) = Address::from_mac(&found.mac_address) else {
                continue;
            };
            let instance = found.object_identifier.instance_number();
            // what we registered ourselves carries vendor 0 until a real I-Am replaces it
            if found.vendor_id == 0 && self.added.contains(&(instance, address)) {
                continue;
            }
            let remote = found.source_network.map(|network| RemoteStation {
                network,
                mac: found
                    .source_address
                    .map(|mac| mac.to_vec())
                    .unwrap_or_default(),
            });
            match self.devices.iter_mut().find(|known| {
                known.instance == instance && known.address == address && known.remote == remote
            }) {
                Some(known) => known.answered_who_is = true,
                None => self.devices.push(Device {
                    instance,
                    address,
                    remote,
                    answered_who_is: true,
                    details: Details::default(),
                    objects: ObjectList::default(),
                }),
            }
        }
    }

    /// One address of the sweep has answered, or not.
    pub fn answer(&mut self, ip: SocketAddrV4, answer: Option<Result<u32, String>>) {
        self.asked += 1;
        let address = Address::Ip(ip);
        match answer {
            Some(Ok(instance)) => {
                let known = self
                    .devices
                    .iter()
                    .any(|device| device.address == address && device.remote.is_none());
                if !known {
                    self.devices.push(Device {
                        instance,
                        address,
                        remote: None,
                        answered_who_is: false,
                        details: Details::default(),
                        objects: ObjectList::default(),
                    });
                }
            }
            Some(Err(reason)) => self.refused.push((ip, reason)),
            None => {}
        }
    }

    /// Devices whose details nobody asked for yet; they count as asked from now on.
    pub fn undetailed(&mut self) -> Vec<Device> {
        let fresh: Vec<Device> = self
            .devices
            .iter()
            .filter(|device| !self.detailed.contains(&(device.instance, device.address)))
            .cloned()
            .collect();
        self.detailed
            .extend(fresh.iter().map(|device| (device.instance, device.address)));
        fresh
    }

    pub fn added(&mut self, device: &Device) {
        self.added.insert((device.instance, device.address));
    }

    pub fn details(&mut self, instance: u32, details: Details) {
        for device in self
            .devices
            .iter_mut()
            .filter(|device| device.instance == instance)
        {
            device.details = details.clone();
        }
    }

    /// How far the object list of a device is read.
    pub fn listing(&mut self, instance: u32, read: usize, total: Option<usize>) {
        for device in self.with_instance(instance) {
            device.objects.read = read;
            device.objects.total = total;
        }
    }

    pub fn object_list(&mut self, instance: u32, list: ObjectList) {
        for device in self.with_instance(instance) {
            device.objects = list.clone();
        }
    }

    fn with_instance(&mut self, instance: u32) -> impl Iterator<Item = &mut Device> {
        self.devices
            .iter_mut()
            .filter(move |device| device.instance == instance)
    }

    pub fn progress(&self, phase: Phase, wanted: impl Fn(u32) -> bool) -> Progress {
        let devices: Vec<Device> = self
            .devices
            .iter()
            .filter(|device| wanted(device.instance))
            .cloned()
            .collect();
        Progress {
            phase,
            found: devices.len(),
            devices,
        }
    }
}

use crate::prelude::*;
use alloc::vec::Vec;
use core::convert::TryInto;
use x86_64::instructions::port::Port;

const PCI_DP: u16 = 0xCFC;
const PCI_CP: u16 = 0xCF8;

pub struct Pci {
    data_port: Port<u32>,
    command_port: Port<u32>,
}

#[derive(Debug)]
pub struct Device {
    bus: u16,
    device: u16,
    function: u16,
    vendor_id: u16,
    device_id: u16,
    class_id: u16,
    subclass_id: u16,
    interface_id: u8,
    revision: u8,
    interrupt: u16,
}

impl Pci {
    pub fn new() -> Self {
        Self {
            data_port: Port::new(PCI_DP),
            command_port: Port::new(PCI_CP),
        }
    }

    fn get_device(&mut self, bus: u16, device: u16, function: u16) -> Device {
        Device {
            bus,
            device,
            function,
            vendor_id: self.read(bus, device, function, 0x00) as u16,
            device_id: self.read(bus, device, function, 0x02) as u16,

            class_id: self.read(bus, device, function, 0x0b) >> 8,
            subclass_id: self.read(bus, device, function, 0x0a) & 0xff,
            interface_id: self.read(bus, device, function, 0x09) as u8,

            revision: self.read(bus, device, function, 0x08) as u8,
            interrupt: self.read(bus, device, function, 0x3c) & 0x00ff,
        }
    }

    pub fn enumerate(&mut self) {
        let mut devices = Vec::new();
        for bus in 0..8 {
            for dev in 0..32 {
                for fnt in 0..8 {
                    let device = self.get_device(bus, dev, fnt);
                    if device.vendor_id <= 0x0004 || device.vendor_id == 0xffff {
                        continue;
                    }

                    devices.push(device);
                }
            }
        }

        devices.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        devices.dedup_by(|b, a| a.device_id.eq(&b.device_id));

        for d in devices {
            sprintln!("{:x?}", d);
        }
    }

    fn get_base_addr_reg(&mut self, bus: u16, device: u16, fun: u16, bar: u16) {}

    fn read(&mut self, bus: u16, device: u16, fun: u16, offset: u32) -> u16 {
        let id: u32 = 0x1 << 31
            | ((bus as u32) << 16 | (device as u32) << 11 | (fun as u32) << 8) as u32
            | (offset & 0xfc);

        unsafe {
            self.command_port.write(id);
        }

        unsafe { (self.data_port.read() >> (8 * (offset & 2)) & 0xffff) as u16 }
    }
}

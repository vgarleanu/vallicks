use crate::prelude::*;
use crate::rtl8139::RTL8139;
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

    port_base: Option<u32>,
}

#[derive(Debug)]
pub struct BaseAddrReg {
    addr: u32,
    size: u32,
    reg_type: DeviceType,
    prefetch: bool,
}

#[derive(Debug)]
pub enum DeviceType {
    MemoryMapping = 0,
    InputOutput = 1,
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
            port_base: None,
        }
    }

    pub fn enumerate(&mut self) {
        let mut devices = Vec::new();
        for bus in 0..8 {
            for dev in 0..32 {
                for fnt in 0..8 {
                    let mut device = self.get_device(bus, dev, fnt);
                    if device.vendor_id <= 0x0004 || device.vendor_id == 0xffff {
                        continue;
                    }

                    for i in 0..6 {
                        let bar = self.get_base_addr_reg(bus, dev, fnt, i);
                        if let Some(x) = bar {
                            match x.reg_type {
                                DeviceType::InputOutput => {
                                    device.port_base = Some(x.addr as u32);
                                }
                                _ => {}
                            }
                        }
                    }

                    devices.push(device);
                }
            }
        }

        devices.sort_by(|a, b| a.device_id.cmp(&b.device_id));

        for d in devices {
            sprintln!("{:x?}", d);

            if d.vendor_id == 0x10ec {
                println!("Found RTL8139 NIC at: {:x}:{:x}", d.vendor_id, d.device_id);
                self.set_mastering(d.bus, d.device, d.function);
                let mut rtl = RTL8139::new(d.port_base.unwrap());
                rtl.init();
            }
        }
    }

    fn get_base_addr_reg(
        &mut self,
        bus: u16,
        device: u16,
        fun: u16,
        bar: u16,
    ) -> Option<BaseAddrReg> {
        let hdr_type = self.read(bus, device, fun, 0x0e) & 0x7f;
        if bar >= 6 - (4 * hdr_type) {
            return None;
        }

        let bar_val = self.read32(bus, device, fun, (0x10 + 4 * bar).into());
        let dev_type = if (bar_val & 0x1) == 1 {
            DeviceType::InputOutput
        } else {
            DeviceType::MemoryMapping
        };

        match dev_type {
            DeviceType::InputOutput => Some(BaseAddrReg {
                addr: (bar_val & 0xfffc) as u32,
                size: 0,
                reg_type: dev_type,
                prefetch: false,
            }),
            _ => None,
        }
    }

    fn set_mastering(&mut self, bus: u16, device: u16, fun: u16) {
        let original_conf = self.read32(bus, device, fun, 0x04);
        let next_conf = original_conf | 0x04;

        let id: u32 = 0x1 << 31
            | ((bus as u32) << 16 | (device as u32) << 11 | (fun as u32) << 8) as u32
            | 0x04;

        unsafe {
            self.command_port.write(id);
            self.data_port.write(next_conf);
        }

        let next = self.read32(bus, device, fun, 0x04);
        println!("   Orign: {:#034b} New: {:#034b}", original_conf, next);
    }

    fn read(&mut self, bus: u16, device: u16, fun: u16, offset: u32) -> u16 {
        let id: u32 = 0x1 << 31
            | ((bus as u32) << 16 | (device as u32) << 11 | (fun as u32) << 8) as u32
            | (offset & 0xfc);

        unsafe {
            self.command_port.write(id);
        }

        unsafe { (self.data_port.read() >> (8 * (offset & 2)) & 0xffff) as u16 }
    }

    fn read32(&mut self, bus: u16, device: u16, fun: u16, offset: u32) -> u32 {
        let id: u32 = 0x1 << 31
            | ((bus as u32) << 16 | (device as u32) << 11 | (fun as u32) << 8) as u32
            | (offset & 0xfc);

        unsafe {
            self.command_port.write(id);
        }

        unsafe { self.data_port.read() }
    }
}

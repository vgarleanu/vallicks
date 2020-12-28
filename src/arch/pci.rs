//! This is the PCI module that needs refactoring
#![allow(missing_docs)]
use crate::prelude::*;
use x86_64::instructions::port::Port;

/// Struct holds a list of pci devices detected on the system
pub struct Pci {
    pub devices: Vec<Device>,
}

/// Struct represents a single PCI device
#[derive(Debug, Clone)]
pub struct Device {
    pub bus: u16,
    pub device: u16,
    pub function: u16,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u16,
    pub subclass_id: u16,
    pub interface_id: u8,
    pub revision: u8,
    pub interrupt: u16,
    pub port_base: Option<u32>,

    data_port: Port<u32>,
    command_port: Port<u32>,
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
        // NOTE: running in debug mode makes this code panic and cause a pagefault
        //       see: https://github.com/phil-opp/blog_os/issues/743
        Self {
            devices: Vec::new(),
        }
    }

    pub fn enumerate(&mut self) {
        println!("pci: Starting enumeration");
        for bus in 0..8 {
            for dev in 0..32 {
                for fnt in 0..8 {
                    if let Some(device) = Device::from(bus, dev, fnt) {
                        self.devices.push(device);
                    }
                }
            }
        }

        self.devices.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        println!("pci: Enumerated {} devices:", self.devices.len());
        for device in self.devices.iter() {
            println!(
                "       Bus: {} Device: {} ID: {:x}:{:x} Class: {:x}:{:x}",
                device.bus,
                device.device,
                device.vendor_id,
                device.device_id,
                device.class_id,
                device.subclass_id
            );
        }
    }

    pub fn find(
        &self,
        class_id: u16,
        subclass_id: u16,
        vendor_id: u16,
        device_id: u16,
    ) -> Option<Device> {
        for device in self.devices.iter() {
            if device.class_id == class_id
                && device.subclass_id == subclass_id
                && device.vendor_id == vendor_id
                && device.device_id == device_id
            {
                return Some(device.clone());
            }
        }
        None
    }
}

impl Device {
    /// Method takes a bus, device and function int and tries to get information about them, if
    /// such a device exists it returns a new instance of Self, otherwise returns None.
    pub fn from(bus: u16, device: u16, function: u16) -> Option<Self> {
        let mut device = Self {
            bus,
            device,
            function,
            data_port: Port::new(0xcfc),
            command_port: Port::new(0xcf8),
            ..Default::default()
        };

        device.fill_headers();

        if device.vendor_id <= 0x0004 || device.vendor_id == 0xffff {
            return None;
        }

        for i in 0..6 {
            if let Some(x) = device.get_base_addr_reg(i) {
                match x.reg_type {
                    DeviceType::InputOutput => {
                        device.port_base = Some(x.addr as u32);
                    }
                    _ => {}
                }
            }
        }

        Some(device)
    }

    fn fill_headers(&mut self) {
        self.vendor_id = self.read(0x00) as u16;
        self.device_id = self.read(0x02) as u16;

        self.class_id = self.read(0x0b) >> 8;
        self.subclass_id = self.read(0x0a) & 0xff;
        self.interface_id = self.read(0x09) as u8;

        self.revision = self.read(0x08) as u8;
        self.interrupt = self.read(0x3c) & 0x00ff;
    }

    fn get_base_addr_reg(&mut self, bar: u16) -> Option<BaseAddrReg> {
        let hdr_type = self.read(0x0e) & 0x7f;

        if bar >= 6 - (4 * hdr_type) {
            return None;
        }

        let bar_val = self.read32((0x10 + 4 * bar).into());

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

    pub fn set_mastering(&mut self) {
        let original_conf = self.read32(0x04);
        let next_conf = original_conf | 0x04;

        unsafe {
            self.command_port.write(self.get_id(0x04));
            self.data_port.write(next_conf);
        }

        println!(
            "pci: Done setting bitmastering for {:x}:{:x}",
            self.vendor_id, self.device_id
        );
    }

    pub fn set_enable_int(&mut self) {
        let original_conf = self.read32(0x04);
        let next_conf = original_conf & !(1 << 10);

        unsafe {
            self.command_port.write(self.get_id(0x04));
            self.data_port.write(next_conf);
        }
    }

    pub fn set_disable_int(&mut self) {
        let original_conf = self.read32(0x04);
        let next_conf = original_conf | (1 << 10);

        unsafe {
            self.command_port.write(self.get_id(0x04));
            self.data_port.write(next_conf);
        }
    }

    pub fn set_interrupt(&mut self, int: u32) {
        unsafe {
            self.command_port.write(self.get_id(0x3c));
            self.data_port.write(int);
        }

        println!(
            "pci: Done setting interrupt to {} for {:x}:{:x}",
            int, self.vendor_id, self.device_id
        );
    }

    fn read(&mut self, offset: u32) -> u16 {
        unsafe {
            self.command_port.write(self.get_id(offset & 0xfc));
            (self.data_port.read() >> (8 * (offset & 2)) & 0xffff) as u16
        }
    }

    fn read32(&mut self, offset: u32) -> u32 {
        unsafe {
            self.command_port.write(self.get_id(offset & 0xfc));
            self.data_port.read()
        }
    }

    fn get_id(&self, offset: u32) -> u32 {
        0x1 << 31
            | (self.bus as u32) << 16
            | (self.device as u32) << 11
            | (self.function as u32) << 8
            | offset
    }
}

impl Default for Device {
    fn default() -> Self {
        Self {
            bus: 0,
            device: 0,
            function: 0,
            data_port: Port::new(0xcfc),
            command_port: Port::new(0xcf8),
            vendor_id: 0,
            device_id: 0,
            class_id: 0,
            subclass_id: 0,
            interface_id: 0,
            revision: 0,
            interrupt: 0,
            port_base: None,
        }
    }
}

use crate::arch::pci::Device;
use crate::prelude::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

pub mod rtl8139;
pub mod serial;
pub mod vga;

lazy_static! {
    pub static ref DRIVERS: Mutex<Vec<Driver>> = Mutex::new(Vec::new());
}

pub enum Driver {
    NetworkDriver(NetworkDriver),
}

pub enum NetworkDriver {
    RTL8139(rtl8139::RTL8139),
}

impl Driver {
    pub fn load(devices: &mut Vec<Device>) {
        for mut device in devices {
            if device.class_id == 0x2 {
                if let Some(x) = NetworkDriver::load(&mut device) {
                    let mut lock = DRIVERS.lock();
                    lock.push(Driver::NetworkDriver(x));
                    println!(
                        "[DRIVER] Loaded driver for {:x}:{:x}",
                        device.vendor_id, device.device_id
                    );
                }
            }
        }
    }
}

impl NetworkDriver {
    fn load(device: &mut Device) -> Option<Self> {
        if device.vendor_id == 0x10ec && device.device_id == 0x8139 && device.subclass_id == 0x00 {
            println!("[DRIVER] Found device RTL8139...attempting to load");
            let port_base = match device.port_base {
                Some(x) => x,
                None => {
                    println!("[DRIVER] Port base not found for 10ec:8139");
                    return None;
                }
            };

            device.set_mastering();
            device.set_enable_int();
            let mut driver = rtl8139::RTL8139::new(port_base);
            driver.init();
            return Some(NetworkDriver::RTL8139(driver));
        }
        None
    }
}

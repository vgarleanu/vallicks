use crate::arch::pci::Device;
use crate::prelude::*;
use alloc::vec::Vec;
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
        for device in devices {
            if device.class_id == 0x2 {
                if let Some(x) = NetworkDriver::load(device.clone()) {
                    let mut lock = DRIVERS.lock();
                    lock.push(Driver::NetworkDriver(x));
                    println!(
                        "driver: Loaded driver for {:x}:{:x}",
                        device.vendor_id, device.device_id
                    );
                }
            }
        }
    }
}

impl NetworkDriver {
    fn load(mut device: Device) -> Option<Self> {
        if device.vendor_id == 0x10ec && device.device_id == 0x8139 && device.subclass_id == 0x00 {
            println!("driver: Found device RTL8139...attempting to load");

            if device.port_base.is_none() {
                println!("driver: Port base not found for 10ec:8139");
                return None;
            }

            device.set_mastering();
            device.set_enable_int();

            let mut driver = rtl8139::RTL8139::new(device);
            driver.init();

            return Some(NetworkDriver::RTL8139(driver));
        }
        None
    }
}

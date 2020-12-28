#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use futures_util::stream::StreamExt;

use vallicks::arch::pit::get_milis;
use vallicks::async_::*;
use vallicks::driver::rtl8139;
use vallicks::prelude::*;

#[entrypoint]
fn main() {
    println!("Ok");
    let device = get_rtl8139_driver();
}

fn get_rtl8139_driver() -> rtl8139::RTL8139 {
    let mut devices = vallicks::arch::pci::Pci::new();
    devices.enumerate();

    if let Some(mut device) = devices.find(0x2, 0x00, 0x10ec, 0x8139) {
        println!("driver: Found device RTL8139...attempting to load");

        if device.port_base.is_none() {
            panic!("driver: Port base not found for 10ec:8139");
        }

        device.set_mastering();
        device.set_enable_int();

        let mut driver = rtl8139::RTL8139::new(device);
        driver.init();
        return driver;
    }

    unreachable!("fuck");
}

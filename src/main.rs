#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use vallicks::arch::pit::get_milis;
use vallicks::async_::*;
use vallicks::driver::rtl8139;
use vallicks::net::wire::eth2::Ether2Frame;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::wire::mac::Mac;
use vallicks::net::NetworkDevice;
use vallicks::net::StreamSplit;
use vallicks::prelude::*;

use core::convert::{From, Into};

#[entrypoint]
fn main() {
    println!("Ok");
    let mut executor = executor::Executor::new();
    executor.spawn(Task::new(send_packets()));
    executor.run();
}

async fn send_packets() {
    let mut device = get_rtl8139_driver();
    let mut eth0 = NetworkDevice::new(&mut device);
    eth0.set_ip(Ipv4Addr::new(192, 168, 100, 51));
    eth0.process().await;
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

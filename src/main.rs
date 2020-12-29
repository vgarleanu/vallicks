#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use vallicks::async_::*;
use vallicks::driver::rtl8139::RTL8139;
use vallicks::driver::Driver;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::NetworkDevice;
use vallicks::prelude::*;

#[entrypoint]
fn main() {
    println!("Ok");
    let mut executor = executor::Executor::new();
    executor.spawn(Task::new(send_packets()));
    executor.run();
}

async fn send_packets() {
    if let Some(mut phy) = RTL8139::probe().and_then(|x| Some(RTL8139::preload(x))) {
        if phy.init().is_ok() {
            let mut netdev = NetworkDevice::new(&mut phy);
            netdev.set_ip(Ipv4Addr::new(192, 168, 100, 51));
            spawn(do_something());
            spawn(async move { netdev.run_forever().await });
        }
    }
}

async fn do_something() {
    println!("lmao");
}

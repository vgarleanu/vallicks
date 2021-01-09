#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::time::Duration;
use futures_util::stream::StreamExt;
use rtl8139_rs::*;
use vallicks::arch::pit::get_milis;
use vallicks::async_::*;
use vallicks::driver::rtl8139::*;
use vallicks::driver::Driver;
use vallicks::net::socks::TcpListener;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::NetworkDevice;
use vallicks::prelude::*;

#[entrypoint]
fn main() {
    println!("Ok");
    let mut executor = executor::Executor::new();

    // run the network stack for our rtl8139 nic as a separate async task.
    executor.spawn(Task::new(netstack_process()));
    executor.spawn(Task::new(tcp_test()));
    executor.run();
}

async fn tcp_test() {
    let mut listener = TcpListener::bind(1234).expect("failed to bind to port 1234");

    println!("listening on 1234");
    loop {
        if let Some(mut conn) = listener.accept().await {
            loop {
                let mut buffer: [u8; 1000] = [0; 1000];
                let read = conn.read(&mut buffer).await;
                if read > 0 {
                    println!("{}", String::from_utf8_lossy(&buffer[..read]));
                    conn.write(&buffer[..read]).await;
                }
            }
        }
    }
}

async fn netstack_process() {
    // probe for the rtl8139 nic and then load.
    let mut phy = RTL8139::probe().map(|x| RTL8139::preload(x)).unwrap();

    // init the driver
    if phy.init().is_err() {
        panic!("failed to start phy");
    }

    let mut netdev = NetworkDevice::new(&mut phy);
    netdev.set_ip(Ipv4Addr::new(192, 168, 100, 51));

    netdev.run_forever().await
}

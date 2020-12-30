#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use vallicks::async_::*;
use vallicks::arch::pit::get_milis;
use vallicks::driver::rtl8139::RTL8139;
use vallicks::driver::Driver;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::NetworkDevice;
use vallicks::prelude::*;
use vallicks::net::wire::eth2::Ether2Frame;

use core::task::Poll;
use core::pin::Pin;

use futures_util::task::AtomicWaker;

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
            let mut sender = netdev.get_sender();
            spawn(async move { netdev.run_forever().await });
        }
    }
}

struct After(u64, u64, AtomicWaker);

impl After {
    pub fn new(every: u64) -> Self {
        let cur = get_milis();

        Self(cur, cur + every, AtomicWaker::new())
    }
}

impl core::future::Future for After {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        if get_milis() >= self.1 {
            return Poll::Ready(());
        }

        self.2.register(&cx.waker());
        self.2.wake();
        Poll::Pending
    }
}

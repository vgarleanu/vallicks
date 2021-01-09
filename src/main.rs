#![no_std]
#![no_main]

use rtl8139_rs::*;
use vallicks::async_::*;
use vallicks::driver::Driver;
use vallicks::net::socks::TcpListener;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::NetworkDevice;
use vallicks::prelude::*;

async fn netstack_init() {
    let mut phy = RTL8139::probe().map(|x| RTL8139::preload(x)).unwrap();

    if phy.init().is_err() {
        panic!("failed to start phy");
    }

    let mut netdev = NetworkDevice::new(&mut phy);
    netdev.set_ip(Ipv4Addr::new(192, 168, 100, 51));

    netdev.run_forever().await
}


async fn tcp_echo_server() {
    let mut listener = TcpListener::bind(1234).expect("failed to bind to port 1234");

    loop {
        if let Some(mut conn) = listener.accept().await {
            spawn(async move {
                loop {
                    let mut buf: [u8; 1000] = [0; 1000];
                    let read = conn.read(&mut buf).await;
                    if read > 0 {
                        println!("{}", String::from_utf8_lossy(&buf[..read]));
                        conn.write(&buf[..read]).await;
                    }
                }
            });
        }
    }
}

#[entrypoint]
fn main() {
    println!("Ok");
    let mut executor = executor::Executor::new();

    executor.spawn(Task::new(netstack_init()));
    executor.spawn(Task::new(tcp_echo_server()));
    executor.run();
}

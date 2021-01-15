#![no_std]
#![no_main]

use rtl8139_rs::*;
use vallicks::async_::*;
use vallicks::driver::Driver;
use vallicks::net::socks::TcpListener;
use vallicks::net::wire::ipaddr::Ipv4Addr;
use vallicks::net::NetworkDevice;
use vallicks::prelude::*;
use vallicks::sync::RwLock;
use vallicks::sync::Arc;
use core::time::Duration;

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

/// Function will incrememnt the value in the lock every second.
async fn test_write(lock: Arc<RwLock<u64>>) {
    let mut prev = 0u64;
    println!("test write started");

    loop {
        {
            let mut value = lock.write().await;
            println!("test write locked");
            prev += 1;
            *value = prev;
        }

        println!("test write going to sleep");
        Sleep::new(Duration::from_millis(1000)).await;
        println!("test write awake");
    }
}

async fn test_read(lock: Arc<RwLock<u64>>) {
    let mut prev = 0u64;

    println!("test read started");
    loop {
        {
            let value = lock.read().await;
            println!("test read locked");
            if *value != prev {
                println!("{}", value);

                prev = *value;
            }
        }

        println!("test read going to sleep");
        Sleep::new(Duration::from_millis(10)).await;
        println!("test read awake");
    }
}

#[entrypoint]
fn main() {
    println!("Ok");
    let mut executor = executor::Executor::new();

    let lock = Arc::new(RwLock::new(0));
    /*
    executor.spawn(Task::new(netstack_init()));
    executor.spawn(Task::new(tcp_echo_server()));
    */
    executor.spawn(Task::new(test_read(lock.clone())));
    executor.spawn(Task::new(test_write(lock.clone())));
    executor.run();
}

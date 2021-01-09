# Vallicks

## What is Vallicks?
Vallicks is a unikernel written in Rust for fun, designed to be used to create microservices that run in the cloud.

## Why another unikernel?
I'm doing this simply for fun. But who knows what might come out out of it.

## Features
- [x] proc-macros for easy bootstrapping
- [ ] SMP
- [x] Async
- [ ] Drivers
    - [ ] Kernel
        - [x] VGA Text mode
        - [x] Keyboard int driver
        - [x] Pic8259 (masked)
        - [x] Pit8259
        - [ ] APIC/x2APIC
        - [ ] ACPI
        - [ ] RTC
    - [ ] Peripherals
        - [x] RTL8139
        - [ ] Virtio
        - [ ] VBE
- [ ] Network Stack
    - [x] ARP
    - [x] ICMP
    - [x] TCP (partial)
    - [ ] UDP
    - [ ] QUIC?
    - [ ] TLS

## Usage - Simple TCP Echo Server
```rust
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
```

## Contributing
Contributions are absolutely, positively welcome and encouraged! Contributions
come in many forms. You could:

  1. Submit a feature request or bug report as an [issue].
  2. Ask for improved documentation as an [issue].
  3. Contribute code via [merge requests].

[issue]: https://github.com/vgarleanu/vallicks/issues
[merge requests]: https://github.com/vgarleanu/vallicks/merge_requests

All pull requests are code reviewed and tested by the CI. Note that unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion in Vallicks by you shall be licensed under the GNU GPLv2 License 
without any additional terms or conditions.

## License
Vallicks is licensed under the GPLv2 license ([LICENSE.md](LICENSE.md) or http://opensource.org/licenses/GPL-2.0)

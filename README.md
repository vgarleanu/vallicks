# Vallicks - The Rust Unikernel powered by naked_std.

## What is Vallicks?
Vallicks is a unikernel written in Rust for fun, designed to be used to create microservices that run in the cloud. Vallicks comes with a standard library that we call naked_std.

## Why another unikernel?
I'm doing this simply for fun. But who knows what might come out out of it.

## Features
- [x] proc-macros for easy bootstrapping
- [x] Multi-threading
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
    - [ ] TCP
    - [ ] UDP
    - [ ] QUIC?
    - [ ] TLS

## Usage - Simple TCP Server
```rust
#![no_std]
#![no_main]

use vallicks::naked_std::*;
use vallicks::prelude::*;

fn handle_client(mut stream: TcpStream) {
    let mut data = [0u8; 50];
    match stream.read(&mut data) {
        Ok(size) => {
            stream.write(&data[0..size]).unwrap();
        }
        Err(_) => {
            println!("Error Occured: {:?}", stream.peer_addr().unwrap());
            break;
        }
    }
}

#[entrypoint]
fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream));
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
```

## Contributing
Contributions are absolutely, positively welcome and encouraged! Contributions
come in many forms. You could:

  1. Submit a feature request or bug report as an [issue].
  2. Ask for improved documentation as an [issue].
  3. Contribute code via [merge requests].

[issue]: https://gitlab.com/vgarleanu/vallicks/issues
[merge requests]: https://gitlab.com/vallicks/merge_requests

All pull requests are code reviewed and tested by the CI. Note that unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion in Vallicks by you shall be licensed under the GNU GPLv2 License 
without any additional terms or conditions.

## License
Vallicks is licensed under the GPLv2 license ([LICENSE.md](LICENSE.md) or http://opensource.org/licenses/GPL-2.0)

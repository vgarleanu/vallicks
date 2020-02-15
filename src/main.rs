#![no_std]
#![no_main]

use vallicks::prelude::*;

#[entrypoint]
fn main() {
    thread::spawn(vallicks::net::stack::net_thread);
}

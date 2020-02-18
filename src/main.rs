#![no_std]
#![no_main]

use vallicks::naked_std::thread;
use vallicks::prelude::*;

fn join_try(i: u32) -> String {
    format!(
        "Hello from thread: {} with {}",
        thread::current().as_u64(),
        i
    )
}

#[entrypoint]
fn main() {
    let mut threads = Vec::new();

    for i in 0..2 {
        threads.push(thread::spawn(move || join_try(i)));
    }

    for i in threads.drain(..) {
        println!("{}", i.join());
    }
}

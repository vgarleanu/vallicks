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
    let mut test = Vec::new();
    for i in 0..1024 {
        test.push(String::from("FUCK FUCK FUCK"));
    }
    let mut threads = Vec::new();

    for i in 0..3 {
        threads.push(thread::spawn(move || join_try(i)));
    }

    for i in threads.drain(..) {
        println!("{:?}", i.join());
    }
}

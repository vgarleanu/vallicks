#![no_std]
#![no_main]

use vallicks::prelude::*;
use vallicks::schedule::thread::JoinHandle;

fn join_try(i: u32) -> String {
    format!("Hello from thread: {}", i)
}

#[entrypoint]
fn main() {
    let h = thread::spawn(|| join_try(12313));
    let val = h.join();
    println!("{}", val);

    let h = thread::spawn(|| join_try(12313));
    println!("{}", h.join());
}

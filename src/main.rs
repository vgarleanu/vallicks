#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use vallicks::naked_std::sync::mpsc;
use vallicks::naked_std::thread;
use vallicks::prelude::*;

#[cfg(test)]
#[entrypoint]
fn main() {
    #[cfg(test)]
    test_main();
}

fn main_t() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for i in 0..100 {
            println!(
                "{:?}",
                tx.send(format!("This msg was sent over a channel {}", i))
            );
        }
    });

    for i in 0..100 {
        println!("{:?}", rx.recv());
    }
}

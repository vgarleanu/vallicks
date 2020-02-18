#![no_std]
#![no_main]

use vallicks::naked_std::sync::mpsc;
use vallicks::naked_std::thread;
use vallicks::prelude::*;

#[entrypoint]
fn main() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for i in 0..100 {
            println!("{:?}", tx.send(format!("This msg was sent over a channel {}", i)));
        }
    });

    for i in 0..100 {
        println!("{:?}", rx.recv());
    }
}

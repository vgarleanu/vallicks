#![no_std]
#![no_main]

use vallicks::prelude::*;

fn join_try() -> String {
    thread::sleep(50);
    String::from("Hello world from another thread")
}

#[entrypoint]
fn main() {
    let handle = thread::spawn(join_try);

    let ret = handle.join();

    println!("{:?}", ret);
}

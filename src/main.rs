#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{format, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rust_kernel::pci;
use rust_kernel::pit::get_milis;
use rust_kernel::prelude::*;

entry_point!(__kmain);

fn menu() {
    loop {
        if let Some(x) = input() {
            print!("{}", x);
        }
    }
}

fn sleep_ever_s() {
    loop {
        println!("Thread {}: {}", thread::current().as_u64(), get_milis());
        thread::sleep(1900); // sleep for 1s
    }
}

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);

    let mut pci = pci::Pci::new();
    pci.enumerate();

    println!("Booted...");

    #[cfg(test)]
    test_main();
    halt();
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    sprintln!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit(ExitCode::Success);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    sprintln!("{}", info);
    halt();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_kernel::test_panic_handler(info);
}

#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rust_kernel::pit::get_milis;
use rust_kernel::prelude::*;

entry_point!(__kmain);

fn function() {
    let mut counter = 0u8;
    let current_thread = thread::current();
    loop {
        thread::sleep(5000);
        if counter == 4 {
            break;
        }
        println!(
            "Hello from thread {} cnt: {}",
            current_thread.as_u64(),
            get_milis()
        );
        counter += 1;
    }
}

fn function2() {
    let current_thread = thread::current();
    let mut counter = 1u8;
    loop {
        if counter == 6 {
            break;
        }
        println!(
            "Hello from thread {} cnt: {}",
            current_thread.as_u64(),
            counter
        );
        counter += 1;
    }
}

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);

    thread::spawn(function);

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

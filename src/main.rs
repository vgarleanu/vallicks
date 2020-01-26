#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rust_kernel::schedule as thread;
#[cfg(test)]
use rust_kernel::{exit, ExitCode};
use rust_kernel::{hlt_loop, print, println, sprintln};

entry_point!(__kmain);

fn function() {
    let mut counter = 0u8;
    loop {
        if counter == 4 {
            break;
        }
        println!("Hello from thread 1 cnt: {}", counter);
        counter += 1;
    }
}

fn function2() {
    let mut counter = 1u8;
    loop {
        if counter == 6 {
            break;
        }
        println!("Hello from thread 2 cnt: {}", counter);
        counter += 1;
    }
}

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);

    thread::spawn(function);
    thread::spawn(function2);
    rust_kernel::activate_sch();
    println!("Booted...");

    #[cfg(test)]
    test_main();
    hlt_loop();
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
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_kernel::test_panic_handler(info);
}

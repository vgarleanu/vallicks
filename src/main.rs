#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
#[cfg(test)]
use rust_kernel::{exit, ExitCode};
use rust_kernel::{hlt_loop, println, sprintln};

entry_point!(__kmain);

fn function() {
    println!("Function");
    hlt_loop();
}

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);
    function();

    #[cfg(test)]
    test_main();
    println!("Booted...");
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

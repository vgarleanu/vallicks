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
use rust_kernel::rtl8139::RTL8139;

entry_point!(__kmain);

fn menu() {
    loop {
        if let Some(x) = input() {
            print!("{}", x);
        }
    }
}

fn sleep_ever_s() {
    let mut pci = pci::Pci::new();
    pci.enumerate();
    let mut rtl = RTL8139::new(0xc000);
    rtl.init();
    loop {
        thread::sleep(1000); // sleep for 1s
        let data = [0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x61, 0x67, 0x67, 0x6f, 0x74, 0x20];
        rtl.write(&data);
        unsafe {
            asm!("int 0x22" ::::);
        }
    }
}

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);

    thread::spawn(sleep_ever_s);

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

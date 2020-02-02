#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![no_std]
#![no_main]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
#[allow(unused_imports)]
use rust_kernel::{
    arch::{pci, pit::get_milis},
    driver::*,
    prelude::*,
};

entry_point!(__kmain);

#[allow(dead_code)]
fn menu() {
    loop {
        if let Some(x) = input() {
            print!("{}", x);
        }
    }
}

/*
fn sleep_ever_s() {
    let mut rtl = RTL8139::new(0xc000);
    rtl.init();
    loop {
        thread::sleep(1000); // sleep for 1s
        let data = [
            0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x61, 0x67, 0x67, 0x6f, 0x74, 0x20,
        ];
        rtl.write(&data);
    }
}
*/

fn __kmain(boot_info: &'static BootInfo) -> ! {
    println!("Booting...");
    rust_kernel::init(boot_info);
    let mut pci = pci::Pci::new();
    pci.enumerate();

    Driver::load(&mut pci.devices);

    //thread::spawn(sleep_ever_s);

    println!("Booted...");

    #[cfg(test)]
    test_main();
    halt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

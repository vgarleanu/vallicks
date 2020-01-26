#![no_std]
#![cfg_attr(test, no_main)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(asm)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]
#![feature(option_expect_none)]
#![feature(raw)]
#![feature(try_trait)]
#![feature(never_type)]
#![feature(global_asm)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

#[cfg(test)]
use bootloader::entry_point;

use bootloader::BootInfo;

#[cfg(test)]
entry_point!(__kmain_test);

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pit;
pub mod prelude;
pub mod schedule;
pub mod serial;
pub mod vga;

use crate::memory::{init as __meminit, BootInfoFrameAllocator};
use crate::schedule::init_scheduler;
use x86_64::{structures::paging::Page, VirtAddr};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

pub fn init(boot_info: &'static BootInfo) {
    gdt::init_gdt();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { __meminit(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Failed to initialize heap");

    init_scheduler(mapper, frame_allocator);

    // FIXME: For some reason initiating the PIT before paging crashes the allocator
    pit::init();
}

pub fn exit(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    sprintln!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit(ExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    sprintln!("[failed]\n");
    sprintln!("Error: {}\n", info);
    exit(ExitCode::Failed);
    hlt_loop();
}

#[cfg(test)]
fn __kmain_test(boot_info: &'static BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

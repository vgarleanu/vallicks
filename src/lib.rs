#![no_std]
#![cfg_attr(test, no_main)]
#![feature(
    abi_x86_interrupt,
    asm,
    alloc_error_handler,
    naked_functions,
    option_expect_none,
    raw,
    try_trait,
    never_type,
    global_asm
)]
extern crate alloc;

pub mod arch;
pub mod driver;
pub mod net;
pub mod prelude;
pub mod schedule;

#[allow(unused_imports)]
use crate::{
    arch::{
        memory::{init as __meminit, BootInfoFrameAllocator},
        pci,
        pit::get_milis,
    },
    driver::*,
    schedule::init_scheduler,
};
use bootloader::BootInfo;
use buddy_system_allocator::{Heap, LockedHeapWithRescue};
use core::panic::PanicInfo;
use prelude::*;
use x86_64::VirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeapWithRescue = LockedHeapWithRescue::new(|heap: &mut Heap| {
    let (start, size) = crate::arch::allocator::extend_heap().expect("Failed to extend heap");

    println!("Extra heap {:#x} with size {:#x}", start, size);
    unsafe {
        heap.add_to_heap(start, start + size);
    }
});

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
    arch::cpu::cpu_info();
    arch::gdt::init_gdt();
    println!("gdt: GDT init done...");

    /* We first create the allocator, because the itnerrupt handlers use some allocations
     * internally
     */
    arch::interrupts::init_idt();
    unsafe { arch::interrupts::PICS.lock().initialize() };
    println!("pic: PIC init done...");

    x86_64::instructions::interrupts::enable();
    println!("int: interrupts enabled");

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { __meminit(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    arch::allocator::init_heap(&mut mapper, &mut frame_allocator).map_or_else(
        |_| panic!("alloc: Failed to initialize heap..."),
        |_| println!("alloc: Allocator init done..."),
    );

    init_scheduler(mapper, frame_allocator);

    // FIXME: For some reason initiating the PIT before paging crashes the allocator
    arch::pit::init();

    let mut pci = pci::Pci::new();
    pci.enumerate();

    Driver::load(&mut pci.devices);
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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

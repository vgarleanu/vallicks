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
    global_asm,
    get_mut_unchecked
)]
#![feature(type_alias_impl_trait)]
extern crate alloc;

pub mod arch;
pub mod driver;
pub(crate) mod globals;
pub mod naked_std;
pub mod net;
pub mod prelude;
pub mod schedule;

#[allow(unused_imports)]
use crate::{
    arch::{
        memory::{init as meminit, BootInfoFrameAllocator},
        pci,
        pit::get_milis,
    },
    driver::*,
    prelude::*,
    schedule::init_scheduler,
};
use bootloader::BootInfo;
use core::panic::PanicInfo;
use prelude::*;
use x86_64::VirtAddr;

#[cfg(not(target_arch = "x86_64"))]
compile_error!("This library can only be used on the x86_64 architecture.");

#[cfg(debug_assertions)]
compile_warning!("PIT will be disabled to default as using the PIT in debug builds causes UB");

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

    arch::pit::init();

    x86_64::instructions::interrupts::enable();
    println!("int: interrupts enabled");

    {
        let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

        let mut lock = globals::MAPPER.lock();
        *lock = Some(unsafe { meminit(phys_mem_offset) });

        let mut lock = globals::FRAME_ALLOCATOR.lock();
        *lock = Some(unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) });
    }

    arch::allocator::extend_heap()
        .map(|(start, size)| globals::extend_alloc_heap(start, size))
        .map_or_else(
            |e| panic!("alloc: Failed to initialize heap...\n{}", e),
            |_| println!("alloc: Allocator init done..."),
        );

    init_scheduler();

    let mut pci = pci::Pci::new();
    pci.enumerate();

    Driver::load(&mut pci.devices);
}

pub fn exit(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0xf4);
    unsafe {
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
    // If current_thread_id is 0 that means that the panic was before the main thread was launched
    // if that is the case we simply want to print and halt, otherwise we inform the scheduler to
    // mark the thread as dirty
    if schedule::current_thread_id().as_u64() != 0 {
        schedule::mark_dirty(format!("{}", info));
    }

    println!("{}", info);
    halt();
}

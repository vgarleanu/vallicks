//! Vallicks
//! Vallicks is a x86_64 unikernel designed to be a drop-in replacement for rust's stdlib, offering
//! a equivalent API that runs on bare metal without the overhead of a full blown operating system.
//!
//! With normal user space programs, the application runs naturally under an Operating System, be
//! it Windows or Linux. With vallicks the App itself is the operating system. An advantage of this
//! is that everything runs in Ring-0 removing a lot of the overhead of syscalls. Vallicks also
//! comes bundled with only necessary drivers, all of which are feature gated, allowing the
//! outputted image to be highly specialized to the running enviroment. Therefore one could say
//! that this unikernel is more of bare metal runtime than anything else.
//!
//! This unikernel is also designed to be as extensible as possible. The kernel abstracts away all
//! driver interaction between the standard library and the end user. Allowing the end coder to
//! drop in whatever driver modules they want. Doing so wont require any alteration whatsoever to
//! normal program code. In turn this project is very similar to IncludeOS.
//!
//! Vallicks comes in two parts. First you have `vallicks::*` itself which contains most/all
//! critical kernel code, usually located in `vallicks::arch`, `vallicks::driver` and
//! `vallicks::schedule`. Most of the kernel code is made private by default to minimize the risk
//! of accidentally calling some critical function that can crash the entire kernel. The second
//! part is what we call `naked_std`.
//!
//! naked_std is the standard library for this kernel which has the aim to be API wise almost
//! identical to the rust standard library. This in turn allowing platform specific code to be
//! easily ported to run under vallicks.
//!
//! Although it all sounds simple enough there are some caveats to using this library. One of them
//! is that we cannot inform `rustc` that this is indeed a standard library. Because of this the
//! main program has to manually import `vallicks::prelude::*` and `vallicks::naked_std`. Secondly
//! we mut mark the application itself as `#[no_std]`. Lastly we need to mark the entry point for
//! the application. The function can still be called `main`, but you have to place the attribute
//! macro `#[entrypoint]` above it.
//!
//! The entrypoint attribute macro does several things. First it informs the bootloader the
//! bootloader what the entrypoint of the kernel is. In our case it will be the `main` function.
//! Next it prepares and boots the kernel by calling the `vallicks::init` function and passing it
//! the `BootInfo` struct which contains some vital information needed for correct operation.
//! The actual body of the function gets moved into a thread which we call the main_thread. The
//! main function then attempts to join the main_thread. If the main_thread panics we return a Qemu
//! failure exit code, otherwise we return a success and halt indefinetely.
//! There are planned ways to specify what the abort behaviour should be, and one of them is to
//! reboot the virtual machine.
//!
//! To illustrate that, here is a example hello world
//! ```rust
//! #[no_std]
//! #[no_main]
//! use vallicks::prelude::*;
//!
//! #[entrypoint]
//! fn main() {
//!     println!("Hello world");
//! }
//! ```
//!
//! That snippet then expands into:
//! ```rust
//! #[no_std]
//! #[no_main]
//! use vallicks::prelude::*;
//!
//! bootloader::entrypoint!(#name);
//! fn main(boot_info: &'static bootloader::BootInfo) -> ! {
//!     println!("Booting... Standy...");
//!     vallicks::init(boot_info);
//!     println!("Booted in {}ms", timer::get_milis());
//!
//!     let main_thread = thread::spawn(|| {
//!         println!("Hello world");
//!     });
//!
//!     match main_thread.join() {
//!         Ok(_) => exit(ExitCode::Success),
//!         Err(_) => exit(ExitCode::Failed),
//!     }
//!
//!     halt();
//! }
//! ```
//! The entrypoint macro makes it convinient to boot up the kernel allowing us to automatically
//! start writing userland code, without the need to manually set up the kernel. In addition to
//! that, the entrypoint macro will make it easier for users to migrate from versions of the kernel
//! where the API has changed in some way.
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
    get_mut_unchecked,
    type_alias_impl_trait,
    prelude_import
)]
#![deny(missing_docs)]
extern crate alloc;

/// The arch module holds the lowlevel initation functions to prepare the CPU for the kernel.
pub mod arch;
/// This module holds some drivers that come with vallicks by default, such as a vbe, vga, serial
/// and rtl8139 NIC driver.
pub mod driver;
/// This module holds the global states required for proper operation by the kernel, these include
/// the allocator, the scheduler and the mapper.
pub(crate) mod globals;
/// This is the standard library used and exposed by vallicks.
pub mod naked_std;
/// This holds  our bare network primitives such as packet structures and parsers.
pub mod net;
/// This is the prelude for our kernel which holds the most basic required methods and macros.
#[prelude_import]
pub mod prelude;
/// This holds all the modules related to the scheduler.
pub(crate) mod schedule;

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
use x86_64::VirtAddr;

#[cfg(not(target_arch = "x86_64"))]
compile_error!("This library can only be used on the x86_64 architecture.");

/// Enum represents a qemu specific VM exit code which is used only in two cases. Within vallicks
/// when running cargo xtest, which lets the test suite know if the test passed or not.
///
/// This is also as the exit code for when the main thread exits, if the main thread panics we send
/// the Failed code, otherwise we send Success.
///
/// TODO: Add more exit codes to better map the exit state of the virtual machine, for example a
///       error code specific to out-of-memory panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExitCode {
    /// Represents successful execution
    Success = 0x10,
    /// Represents failed execution
    Failed = 0x11,
}

/// Function is used to handle allocator errors, these errors are usually OOM errors. There is no
/// recovery from such a error so we just panic and never return from it.
///
/// The reason we use println!() and halt here instead of just panic! is because if we get a
/// allocation error that came from a thread, we dont want to just lock the thread, but the entire
/// kernel as this should be a considered a real kernel panic.
///
/// # Arguments
/// `layout` - Contains the layout information for the failed allocation
///
/// TODO: Maybe create a kpanic!() macro that avoids the whole panic! issue?
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    println!("KERNEL PANIC!!! Allocation error: {:?}", layout);
    halt();
}

/// Method boots up and initiates all the required kernel structures. This method can be ignored if
/// you are using the `#[entrypoint]` attribute macro which does all the startup sequences for you.
/// However you must ensure that you dont call this method a second time
///
/// # Sequences
/// 1. First we initiate the GDT
/// 2. We initate the IDT
/// 3. We initiate the PIC and the PIT
/// 4. After we turn interrupts on as we are ready to receive timer and exception interrupts
/// 5. We set up paging and the heap allocator which will allow all code to create allocations
/// 6. Next, we boot up the scheduler allowing us to use all the `naked_std::thread::*` primitives
/// 7. Lastly we scan for all the PCI devices and load up the drivers for each device
///
/// After the init sequences are completed, it is safe to essentially run application level code
/// and use the `naked_std` library.
///
/// # Arguments
/// * `boot_info` - This contains critical boot info and memory maps
pub fn init(boot_info: &'static BootInfo) {
    arch::cpu::cpu_info();
    arch::gdt::init_gdt();
    println!("gdt: GDT init done...");

    /* We first create the allocator, because the interrupt handlers use some allocations
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

/// Method informs Qemu of the status of the VM, allowing us to send error codes downstream. This
/// is only used internally in two places which includes unit tests and the main entrypoint of our
/// virtual machine.
///
/// # Arguments
/// * `exit_code` - The exit code we wish to send to QEMU represented by the ExitCode enum
///
/// NOTE: Maybe we should have this method never return to avoid bogus exits?
pub fn exit(exit_code: ExitCode) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0xf4);
    unsafe {
        port.write(exit_code as u32);
    }
}

/// Simple looping halt instruction used in functions that should never return such as bare thread
/// closures or our panic handlers.
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// This is the panic handler for our unikernel, besides simply printing our panic info, it does
/// one more important thing. Because threads can also panic which in-turn gets them to enter a
/// infinite never ending halt loop, we want to signal to the scheduler that this thread has
/// panic'd.
///
/// Threads that have panic'd are called dirty threads. To detect panicking threads, we first grab
/// the current thread_id, if the thread_id is not 0 it means that a thread is panicking, we then
/// inform the scheduler that the current thread has panic'd and give it the panic info, the
/// scheduler then sends the error downstream and the panic handler halts.
///
/// Once the thread is marked as dirty, the scheduler will instantly free up its stack, resources
/// and remove it from the scheduling queue, at this point this thread will never ever execute
/// another instruction.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let thread_id = schedule::current_thread_id().as_u64();
    // If current_thread_id is 0 that means that the panic was before the main thread was launched
    // if that is the case we simply want to print and halt, otherwise we inform the scheduler to
    // mark the thread as dirty
    if thread_id != 0 {
        // We print and halt here not only for better panic message but to remove a race condition
        // where the dirty thread gets freed before we can print the panic info
        println!("thread {} has panic'd with {}", thread_id, info);
        schedule::mark_dirty(format!("{}", info));
        halt();
    }

    println!("{}", info);
    halt();
}

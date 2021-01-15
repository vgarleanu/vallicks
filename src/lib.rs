//! Vallicks
//! Vallicks is a x86_64 unikernel designed to be used by applications that want to run on bare
//! metal but dont want to write a lot of low level code. In Vallicks theres no need to write low
//! level unsafe code as everything we might need is abstracted behind familiar APIs.
//!
//! With normal user space programs, the application runs naturally under an Operating System, be
//! it Windows or Linux. With vallicks the App itself is the operating system. An advantage of this
//! is that everything runs in Ring-0 removing a lot of the overhead of syscalls. Vallicks offers
//! clear abstractions and traits needed to implement various drivers such as network drivers. All
//! of the driver must be async.
//!
//! Although it all sounds simple enough there are some caveats to using this library. One of them
//! is that we cannot inform `rustc` that this is indeed a standard library. Because of this the
//! main program has to manually import `vallicks::prelude::*`.
//!
//! Secondly we mut mark the application itself as `#[no_std]`. Lastly we need to mark the entry point
//! for the application. The function can still be called `main`, but you have to place the attribute
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
//!     println!("Hello world");
//!
//!     halt();
//! }
//! ```
//! The entrypoint macro makes it convinient to boot up the kernel allowing us to automatically
//! start writing userland code, without the need to manually set up the kernel. In addition to
//! that, the entrypoint macro will make it easier for users to migrate from versions of the kernel
//! where the API has changed in some way.
//!
//! # Using std::*
//! To use the standard library provided with vallicks you must import the prelude, after which you
//! can use the standard library as you would in normal std mode except by replacing `std` with
//! `naked_std`. For example lets spin up a TcpServer:
//! ```rust
//! use vallicks::prelude::*; // import our prelude containing basic imports
//! use vallicks::async_::*; // stuff like our async executor.
//! use rtl8139_rs::*; // RTL8139 nic driver
//! use vallicks::driver::Driver;
//! use vallicks::net::socks::TcpListener;
//! use vallicks::net::wire::ipaddr::Ipv4Addr;
//! use vallicks::net::NetworkDevice;
//!
//! // This function will initiate our NIC and start processing incoming data.
//! async fn netstack_init() {
//!     let mut phy = RTL8139::probe().map(|x| RTL8139::preload(x)).unwrap();
//!
//!     if phy.init().is_err() {
//!         panic!("failed to init phy");
//!     }
//!
//!     let mut netdev = NetworkDevice::new(&mut phy);
//!     netdev.set_ip(Ipv4Addr::new(192, 168, 100, 51)); // set our static ip
//!     netdev.run_forever().await // forever process incoming data.
//! }
//!
//! async fn echo_server() {
//!     let mut listener = TcpListener::bind(1234).expect("failed to bind to port 1234");
//!
//!     loop {
//!         if let Some(mut conn) = listener.accept().await {
//!             async_::spawn(async move {
//!                 loop {
//!                     let mut buf: [u8; 1000] = [0; 1000];
//!
//!                     let read = conn.read(&mut buf).await;
//!                     if read > 0 {
//!                         println!("{}", String::from_utf8_lossy(&buf[..read]);
//!                         conn.write(&buf[..read]).await;
//!                     }
//!                 }
//!             });
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let mut executor = executor::Executor::new();
//!
//!     executor.spawn(Task::new(netstack_init()));
//!     executor.spawn(Task::new(echo_server()));
//!
//!     executor.run();
//! }
//! ```
#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, allow(unused_variables))]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, allow(unused_imports))]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(incomplete_features)]
#![feature(
    abi_x86_interrupt,
    alloc_error_handler,
    alloc_layout_extra,
    allocator_api,
    allocator_internals,
    allow_internal_unsafe,
    arbitrary_self_types,
    array_error_internals,
    asm,
    associated_type_bounds,
    atomic_mut_ptr,
    box_syntax,
    cfg_target_thread_local,
    char_error_internals,
    concat_idents,
    const_raw_ptr_deref,
    const_generics,
    container_error_extra,
    core_intrinsics,
    custom_test_frameworks,
    decl_macro,
    doc_cfg,
    doc_keyword,
    doc_masked,
    doc_spotlight,
    dropck_eyepatch,
    duration_constants,
    exact_size_is_empty,
    exhaustive_patterns,
    external_doc,
    fn_traits,
    format_args_nl,
    generator_trait,
    get_mut_unchecked,
    global_asm,
    hashmap_internals,
    int_error_internals,
    int_error_matching,
    integer_atomics,
    lang_items,
    link_args,
    linkage,
    log_syntax,
    map_first_last,
    maybe_uninit_ref,
    maybe_uninit_slice,
    naked_functions,
    needs_panic_runtime,
    never_type,
    nll,
    option_expect_none,
    panic_info_message,
    panic_internals,
    prelude_import,
    ptr_internals,
    raw,
    rustc_attrs,
    rustc_private,
    shrink_to,
    slice_concat_ext,
    slice_internals,
    std_internals,
    stdsimd,
    stmt_expr_attributes,
    str_internals,
    test,
    thread_local,
    toowned_clone_into,
    trace_macros,
    try_reserve,
    try_trait,
    type_alias_impl_trait,
    type_ascription,
    unboxed_closures,
    untagged_unions,
    unwind_attributes,
    vec_into_raw_parts,
    wake_trait
)]

extern crate alloc;

/// The arch module holds the lowlevel initation functions to prepare the CPU for the kernel.
pub mod arch;
/// The async module holds all the code necessary for async/await support
pub mod r#async;
/// This module holds some drivers that come with vallicks by default, such as a vbe, vga, serial
/// and rtl8139 NIC driver.
pub mod driver;
/// This module holds the global states required for proper operation by the kernel, these include
/// the allocator, the scheduler and the mapper.
pub(crate) mod globals;
/// This holds  our bare network primitives such as packet structures and parsers.
pub mod net;
/// This is the prelude for our kernel which holds the most basic required methods and macros.
#[prelude_import]
pub mod prelude;
/// Holds synchronization primitives.
pub mod sync;

pub use crate::r#async as async_;

#[allow(unused_imports)]
use crate::{
    arch::{
        memory::{init as meminit, BootInfoFrameAllocator},
        pci,
        pit::get_milis,
    },
    driver::*,
    prelude::{compile_warning, format, halt},
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

    arch::pit::init(1000); // start at 1khz

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

    x86_64::instructions::interrupts::enable();
    println!("int: interrupts enabled");
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
    #[cfg(test)]
    uprint!("    ....FAILED!!!\n");
    #[cfg(test)]
    uprint!("{}\n", info);

    #[cfg(not(test))]
    println!("{}", info);

    #[cfg(test)]
    exit(ExitCode::Failed);

    halt();
}

#[cfg(test)]
use bootloader::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main);

/// This is our testing entry point
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop();
}

/// This is our test runner
pub fn test_runner(tests: &[&dyn Fn()]) {
    #[cfg(test)]
    uprint!("\nRunning {} tests\n", tests.len());
    for test in tests {
        test();
    }
    #[cfg(test)]
    uprint!("\nDone testing: {}/{} OK\n", tests.len(), tests.len());
    exit(ExitCode::Success);
}

pub mod allocator;
pub mod cpu;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pci;
pub mod pit;

use x86_64::registers::control::{Cr0, Cr0Flags};

pub(super) fn enable_fpu_on_cpuid() {
    return;
    unsafe {
        Cr0::write(Cr0Flags::EMULATE_COPROCESSOR);
        Cr0::write(Cr0Flags::MONITOR_COPROCESSOR);
    }
}

//! Module planned to be used for checking for specific features and enabling them
use crate::arch::enable_fpu_on_cpuid;
use crate::prelude::*;
use x86::cpuid::CpuId;

/// Checks if the current CPU offers some features and prints to the console if that is the case
pub fn cpu_info() {
    let cpuid = CpuId::new();
    let vendor = cpuid
        .get_vendor_info()
        .expect("Failed to extract CPU vendor");
    let features = cpuid
        .get_feature_info()
        .expect("Failed to extract CPU features");

    println!("Vendor: {}", vendor);
    print!("      CPU features:");

    if features.has_sse3() {
        print!(" sse3");
    }
    if features.has_cmpxchg16b() {
        print!(" cmdpxchg16b");
    }
    if features.has_fpu() {
        print!(" fpu");
        enable_fpu_on_cpuid();
    }
    if features.has_de() {
        print!(" de");
    }
    if features.has_pse() {
        print!(" pse");
    }
    if features.has_tsc() {
        print!(" tsc");
    }
    if features.has_msr() {
        print!(" msr");
    }
    if features.has_mce() {
        print!(" mce");
    }
    if features.has_cmpxchg8b() {
        print!(" cmdpxchg8b");
    }
    if features.has_apic() {
        print!(" apic");
    }
    if features.has_sysenter_sysexit() {
        print!(" sysenter sysexit");
    }
    if features.has_acpi() {
        print!(" acpi");
    }
    if features.has_mmx() {
        print!(" mmx");
    }
    if features.has_sse() {
        print!(" sse");
    }
    if features.has_sse2() {
        print!(" sse2");
    }
    println!();
}

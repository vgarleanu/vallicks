use crate::prelude::*;
use x86::cpuid::CpuId;

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
    if features.has_pclmulqdq() {
        print!(" pclmulqdq");
    }
    if features.has_cpl() {
        print!(" cpl");
    }
    if features.has_vmx() {
        print!(" vmx");
    }
    if features.has_smx() {
        print!(" smx");
    }
    if features.has_eist() {
        print!(" eist");
    }
    if features.has_tm2() {
        print!(" tm2");
    }
    if features.has_ssse3() {
        print!(" ssse3");
    }
    if features.has_cnxtid() {
        print!(" cnxtid");
    }
    if features.has_fma() {
        print!(" fma");
    }
    if features.has_cmpxchg16b() {
        print!(" cmdpxchg16b");
    }
    if features.has_pdcm() {
        print!(" pdcm");
    }
    if features.has_pcid() {
        print!(" pcid");
    }
    if features.has_dca() {
        print!(" dca");
    }
    if features.has_sse41() {
        print!(" sse41");
    }
    if features.has_sse42() {
        print!(" sse42");
    }
    if features.has_x2apic() {
        print!(" x2apic");
    }
    if features.has_movbe() {
        print!(" movbe");
    }
    if features.has_popcnt() {
        print!(" popcnt");
    }
    if features.has_tsc_deadline() {
        print!(" tsc_deadline");
    }
    if features.has_aesni() {
        print!(" aesni");
    }
    if features.has_xsave() {
        print!(" xsave");
    }
    if features.has_oxsave() {
        print!(" oxsave");
    }
    if features.has_avx() {
        print!(" avx");
    }
    if features.has_f16c() {
        print!(" f16c");
    }
    if features.has_rdrand() {
        print!(" rdrand");
    }
    if features.has_fpu() {
        print!(" fpu");
    }
    if features.has_vme() {
        print!(" vme");
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
        print!(" APIC");
    }
    if features.has_sysenter_sysexit() {
        print!(" SYSENTER/SYSEXIT");
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

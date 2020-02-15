use crate::schedule::thread::ThreadId;
use crate::schedule::SCHEDULER;
use alloc::boxed::Box;
use core::mem;
use core::raw::TraitObject;
use x86_64::VirtAddr;

global_asm!(
    "
    .intel_syntax noprefix

    // asm_context_switch(stack_pointer: u64, thread_id: u64)
    asm_context_switch:
        pushfq

        mov rax, rsp
        mov rsp, rdi

        mov rdi, rax
        call add_paused_thread

        popfq
        ret
"
);

pub unsafe fn context_switch_to(thread_id: ThreadId, stack_pointer: VirtAddr) {
    asm!(
        "call asm_context_switch"
        :
        : "{rdi}"(stack_pointer), "{rsi}"(thread_id)
        : "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rpb", "r8", "r9", "r10",
        "r11", "r12", "r13", "r14", "r15", "rflags", "memory"
        : "intel", "volatile"
    );
}

#[no_mangle]
pub extern "C" fn add_paused_thread(paused_stack_pointer: VirtAddr, new_thread_id: ThreadId) {
    let mut lock = SCHEDULER.lock();
    let _ = lock
        .as_mut()
        .expect("scheduler: scheduler not init...")
        .add_paused_thread(paused_stack_pointer, new_thread_id);
}

#[naked]
pub fn call_closure_entry() -> ! {
    unsafe {
        asm!("
        pop rsi
        pop rdi
        call call_closure
    " ::: "mem" : "intel", "volatile")
    };
    unreachable!("call_closure_entry");
}

#[no_mangle]
extern "C" fn call_closure(data: *mut (), vtable: *mut ()) -> ! {
    let trait_object = TraitObject { data, vtable };
    let f: Box<dyn FnOnce() -> !> = unsafe { mem::transmute(trait_object) };
    f()
}

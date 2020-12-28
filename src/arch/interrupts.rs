#![allow(missing_docs)]
use crate::{
    arch::gdt,
    arch::memory::translate_addr,
    arch::pit::tick,
    prelude::{sync::Mutex, *},
};
use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

// Create a PIC instance masking all the interrupts for both pics, meaning all interrupts will be
// sent and setting the offsets from 32 to 32 + 8
pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(32, 40, 0xff, 0xff) });

lazy_static! {
    static ref IDT: Mutex<InterruptDescriptorTable> = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.simd_floating_point.set_handler_fn(handle_fpu_fault);
        idt.x87_floating_point.set_handler_fn(handle_fpu_fault);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX as u16);
        }

        idt[32].set_handler_fn(exception_irq0);

        idt[33].set_handler_fn(__default::<33>);
        idt[34].set_handler_fn(__default::<34>);
        idt[35].set_handler_fn(__default::<35>);
        idt[36].set_handler_fn(__default::<36>);
        idt[37].set_handler_fn(__default::<37>);
        idt[38].set_handler_fn(__default::<38>);
        idt[39].set_handler_fn(__default::<39>);
        idt[40].set_handler_fn(__default::<40>);
        idt[41].set_handler_fn(__default::<41>);
        idt[42].set_handler_fn(__default::<42>);
        idt[43].set_handler_fn(__default::<43>);
        idt[44].set_handler_fn(__default::<44>);
        idt[45].set_handler_fn(__default::<45>);
        idt[46].set_handler_fn(__default::<46>);
        idt[47].set_handler_fn(__default::<47>);
        idt[48].set_handler_fn(__default::<48>);

        Mutex::new(idt)
    };
}

/// Sets up the Interrupt Descriptor Table with all of our exception handles and generic interrupt
/// handlers.
pub fn init_idt() {
    unsafe { IDT.lock().load_unsafe() };
    println!("idt: Interrupt setup done...");
}

/// Allows some module to hook a interrupt by providing a function to be called to this function.
/// It then inserts that function into the global interrupt table. When our generic handler gets
/// called it looks up the interrupt ID in the table and calls the function.
///
/// This allows our modules to hotswap interrupt handlers when required.
pub fn register_interrupt(
    interrupt: usize,
    handler: for<'r> extern "x86-interrupt" fn(&'r mut InterruptStackFrame),
) {
    // set our new interrupt handler
    IDT.lock()[interrupt].set_handler_fn(handler);
    // reload the idt
    unsafe { IDT.lock().load_unsafe() };
}

pub fn notify_eoi(int: u8) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(int);
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Exception: Breakpoint \n{:#?}", stack_frame);
}

// TODO: Forward page faults to offending threads, display a message and then halt and free the
// thread.
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr = Cr2::read();
    println!("Exception: PAGE FAULT");
    println!("Accessed Addr: {:?} Phys: {:?}", addr, unsafe {
        translate_addr(addr)
    });
    println!("Err code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    println!("{:?}", unsafe { translate_addr(stack_frame.stack_pointer) });

    panic!("PageFault, Tried to access: {:#x?}", addr);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn handle_fpu_fault(_: &mut InterruptStackFrame) {
    panic!("EXCEPTION: FPU Fault");
}

extern "x86-interrupt" fn exception_irq0(_: &mut InterruptStackFrame) {
    tick();
    unsafe {
        PICS.lock().notify_end_of_interrupt(32);
    }
}

extern "x86-interrupt" fn __default<const IRQ: u8>(_: &mut InterruptStackFrame) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(IRQ);
    }
}

use crate::{gdt, pit::tick, prelude::*, schedule::schedule};
use arraydeque::{ArrayDeque, Wrapping};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use pic8259_simple::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::{interrupts::without_interrupts, port::Port},
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
pub const NIC: u8 = PIC_1_OFFSET + 2;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
    );
    static ref INPUT_BUFFER: Mutex<ArrayDeque<[char; 64], Wrapping>> =
        Mutex::new(ArrayDeque::new());
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[32].set_handler_fn(exception_irq0);
        idt[NIC as usize].set_handler_fn(nic_irq);
        idt
    };
}

pub fn pop_buffer() -> Option<char> {
    INPUT_BUFFER.lock().pop_front()
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Exception: Breakpoint \n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!("Exception: PAGE FAULT");
    println!("Accessed Addr: {:?}", Cr2::read());
    println!("Err code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    halt();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _err_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    if let Ok(Some(key_event)) = keyboard.add_byte(unsafe { port.read() }) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(c) => {
                    // NOTE: Current VGA output only supports chars within ascii range
                    if !c.is_ascii() {
                        return;
                    }

                    unsafe {
                        INPUT_BUFFER.force_unlock();
                    }
                    let mut lock = INPUT_BUFFER.lock();
                    lock.push_back(c);
                }
                DecodedKey::RawKey(k) => {}
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// TODO: Add our scheduler invocation here
extern "x86-interrupt" fn exception_irq0(_: &mut InterruptStackFrame) {
    tick();
    unsafe {
        PICS.lock().notify_end_of_interrupt(32);
    }
    schedule();
}

// TODO: Assign this interrupt when PCI devices load
extern "x86-interrupt" fn nic_irq(_: &mut InterruptStackFrame) {
    let mut lel: Port<u32> = Port::new(0xc000 + 0x3e);
    println!("Handled some shit");
    unsafe {
        lel.write(0x1);
        PICS.lock().notify_end_of_interrupt(NIC);
    }
}

#[test_case]
fn test_breakpoint_exception() {
    sprint!("test_breakpoint_exception...");
    x86_64::instructions::interrupts::int3();
    sprintln!("[OK]");
}

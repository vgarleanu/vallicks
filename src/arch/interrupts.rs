use crate::{
    arch::gdt,
    arch::memory::translate_addr,
    arch::pit::tick,
    prelude::{
        sync::{Mutex, RwLock},
        *,
    },
    schedule::schedule,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

macro_rules! make_int_handler {
    ($name:ident => $int:expr) => {
        extern "x86-interrupt" fn $name(_frame: &mut InterruptStackFrame) {
            let lock = INT_TABLE.read();
            if let Some(x) = lock.get(&$int) {
                x();
            }
            unsafe {
                PICS.lock().notify_end_of_interrupt($int);
            }
        }
    };
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref INT_TABLE: RwLock<HashMap<i32, Box<dyn Fn() + Send + Sync + 'static>>> =
        RwLock::new(HashMap::with_capacity(16));
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX as u16);
        }
        idt[32].set_handler_fn(exception_irq0);
        idt[33].set_handler_fn(int33);
        idt[34].set_handler_fn(int34);
        idt[35].set_handler_fn(int35);
        idt[36].set_handler_fn(int36);
        idt[37].set_handler_fn(int37);
        idt[38].set_handler_fn(int38);
        idt[39].set_handler_fn(int39);
        idt[40].set_handler_fn(int40);
        idt[41].set_handler_fn(int41);
        idt[42].set_handler_fn(int42);
        idt[43].set_handler_fn(int43);
        idt[44].set_handler_fn(int44);
        idt[45].set_handler_fn(int45);
        idt[46].set_handler_fn(int46);
        idt[47].set_handler_fn(int47);
        idt[48].set_handler_fn(int48);
        idt
    };
}

pub fn register_interrupt<T>(interrupt: i32, handler: T)
where
    T: Fn() + Send + Sync + 'static,
{
    let mut lock = INT_TABLE.write();
    lock.insert(interrupt, Box::new(handler));
}

pub fn init_idt() {
    IDT.load();
    println!("idt: Interrupt setup done...");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("Exception: Breakpoint \n{:#?}", stack_frame);
}

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
    halt();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn exception_irq0(_: &mut InterruptStackFrame) {
    tick();
    unsafe {
        PICS.lock().notify_end_of_interrupt(32);
    }
    schedule();
}

make_int_handler!(int33 => 33);
make_int_handler!(int34 => 34);
make_int_handler!(int35 => 35);
make_int_handler!(int36 => 36);
make_int_handler!(int37 => 37);
make_int_handler!(int38 => 38);
make_int_handler!(int39 => 39);
make_int_handler!(int40 => 40);
make_int_handler!(int41 => 41);
make_int_handler!(int42 => 42);
make_int_handler!(int43 => 43);
make_int_handler!(int44 => 44);
make_int_handler!(int45 => 45);
make_int_handler!(int46 => 46);
make_int_handler!(int47 => 47);
make_int_handler!(int48 => 48);

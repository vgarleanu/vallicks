/// Integrations for rtl8139-rs with this kernel
use crate::arch::interrupts::register_interrupt;
use crate::arch::memory::translate_addr;
use crate::arch::pci::Device;
use crate::net::wire::mac::Mac;
use crate::prelude::*;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::PhysAddr;
use x86_64::VirtAddr;

use crate::driver::Driver;
use crate::driver::NetworkDriver;

use rtl8139_rs::*;

const IRQ: usize = 43;

fn __translate_addr(virt: VirtAddr) -> PhysAddr {
    unsafe { translate_addr(virt).expect("rtl8139: failed to translate virtaddr to physaddr") }
}

impl Driver for RTL8139 {
    type Return = Result<(), ()>;

    fn probe() -> Option<Device> {
        let mut devices = crate::arch::pci::Pci::new();
        devices.enumerate();

        let device = devices.find(0x2, 0x00, 0x10ec, 0x8139);

        if device.is_some() {
            println!("rtl8139: probing positive");
        } else {
            println!("rtl8139: probing negative");
        }

        device
    }

    fn preload(mut device: Device) -> Self {
        // initialize the device.
        device.set_mastering();
        device.set_enable_int();

        let base = device.port_base.unwrap() as u16;
        let this = unsafe { Self::preload_unchecked(base, __translate_addr) };

        extern "x86-interrupt" fn _blanket(_: &mut InterruptStackFrame) {
            RTL8139::on_interrupt();
            crate::arch::interrupts::notify_eoi(IRQ as u8);
        }

        register_interrupt(IRQ, _blanket);

        this
    }

    fn init(&mut self) -> Self::Return {
        self.load()
    }
}

impl NetworkDriver for RTL8139 {
    type RxSink = RxSink;
    type TxSink = TxSink;

    fn parts(&mut self) -> (Self::RxSink, Self::TxSink) {
        self.parts()
    }

    fn mac(&self) -> Mac {
        self.mac().into()
    }
}

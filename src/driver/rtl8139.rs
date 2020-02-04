use crate::arch::{interrupts::register_interrupt, memory::translate_addr, pci::Device};
use crate::prelude::*;
use alloc::{boxed::Box, sync::Arc};
use spin::RwLock;
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

struct RTL8139Inner {
    pub config_1: Port<u32>,
    pub cmd_reg: Port<u8>,
    pub rbstart: Port<u32>,
    pub imr: Port<u16>,
    pub wrap: Port<u32>,
    pub buffer: Box<&'static [u8]>,
    pub tx_dat: [Port<u32>; 4],
    pub tx_cmd: [Port<u32>; 4],
    pub current: usize,
    pub tppoll: Port<u8>,
    pub ack: Port<u16>,
    pub device: Device,
}

pub struct RTL8139 {
    inner: Arc<RwLock<RTL8139Inner>>,
}

impl RTL8139 {
    pub fn new(device: Device) -> Self {
        let base = device.port_base.unwrap() as u16;
        let inner = RTL8139Inner {
            config_1: Port::new(base + 0x52),
            cmd_reg: Port::new(base + 0x37),
            rbstart: Port::new(base + 0x30),
            imr: Port::new(base + 0x3c),
            wrap: Port::new(base + 0x44),
            buffer: Box::new(&[0u8; 8192 + 16]),
            tx_dat: [
                Port::new(base + 0x20),
                Port::new(base + 0x24),
                Port::new(base + 0x28),
                Port::new(base + 0x2c),
            ],
            tx_cmd: [
                Port::new(base + 0x10),
                Port::new(base + 0x14),
                Port::new(base + 0x18),
                Port::new(base + 0x1c),
            ],
            current: 0usize,
            tppoll: Port::new(base + 0xd9),
            ack: Port::new(base + 0x3e),
            device,
        };

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn init(&mut self) {
        let mut inner = self.inner.write();
        println!("rtl8139: Config start");
        unsafe {
            inner.config_1.write(0x0);
            inner.cmd_reg.write(0x10);
        }

        loop {
            if (unsafe { inner.cmd_reg.read() } & 0x10) == 0 {
                break;
            }
        }

        let ptr = VirtAddr::from_ptr(inner.buffer.as_ptr());
        let physical = unsafe { translate_addr(ptr).unwrap() };
        println!(
            "rtl8139: Setting RX buffer to VirtAddr: {:?} PhysAddr: {:?}",
            ptr, physical
        );

        unsafe {
            inner.rbstart.write(physical.as_u64() as u32);
            inner.imr.write(0x809f); // 0x0005 to only handle ROK | TOK
            inner.wrap.write(0xf | (1 << 7));
            inner.cmd_reg.write(0x0c);
        }

        println!("rtl8139: Config done...");
        println!("rtl8139: Registering interrupt handler");
        // FIXME: Figure out a way to remove the double clone here
        let inner_clone = self.inner.clone();
        register_interrupt(43, move || Self::handle_int(inner_clone.clone()));
        println!("rtl8139: Registered interrupt handler");
    }

    pub fn write(&mut self, data: &[u8]) {
        // FIXME: Deadlock occurs if we dont disable and then re-enable interrupts after a write to
        //        device. Figure out a way to avoid these.
        x86_64::instructions::interrupts::disable();
        {
            let current = {
                let reader = self.inner.read();
                reader.current
            };

            let mut inner = self.inner.write();
            let ptr = VirtAddr::from_ptr(data.as_ptr());
            let physical = unsafe { translate_addr(ptr).unwrap() }.as_u64() as u32;

            unsafe {
                inner.tx_dat[current].write(physical);
                inner.tx_cmd[current].write((data.len() as u32) & 0xfff);

                loop {
                    if (inner.tx_cmd[current].read() & 0x8000) != 0 {
                        break;
                    }
                }
            }

            inner.current = (current + 1) % 4;
        }
        x86_64::instructions::interrupts::enable();
    }

    fn handle_int(inner: Arc<RwLock<RTL8139Inner>>) {
        let mut inner = inner.write();
        let isr = unsafe { inner.ack.read() };
        println!("ISR: {:#018b}", isr);
        unsafe {
            inner.ack.write(0xff);
        }
        let isr = unsafe { inner.ack.read() };
        println!("ISR: {:#018b}", isr);
    }
}

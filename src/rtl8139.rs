use crate::memory::translate_addr;
use crate::prelude::*;
use alloc::boxed::Box;
use core::convert::TryInto;
use x86_64::instructions::port::Port;
use x86_64::{PhysAddr, VirtAddr};

pub struct RTL8139 {
    config_1: Port<u32>,
    cmd_reg: Port<u8>,
    rbstart: Port<u32>,
    imr: Port<u16>,
    wrap: Port<u32>,
    buffer: Box<&'static [u8]>,
    tx_dat: [Port<u32>; 4],
    tx_cmd: [Port<u32>; 4],
    current: usize,
    tppoll: Port<u8>,
}

impl RTL8139 {
    pub fn new(base: u32) -> Self {
        Self {
            config_1: Port::new((base as u16) + 0x52),
            cmd_reg: Port::new((base as u16) + 0x37),
            rbstart: Port::new((base as u16) + 0x30),
            imr: Port::new((base as u16) + 0x3c),
            wrap: Port::new((base as u16) + 0x44),
            buffer: Box::new(&[0u8; 8192 + 16]),
            tx_dat: [
                Port::new((base as u16) + 0x20),
                Port::new((base as u16) + 0x24),
                Port::new((base as u16) + 0x28),
                Port::new((base as u16) + 0x2c),
            ],
            tx_cmd: [
                Port::new((base as u16) + 0x10),
                Port::new((base as u16) + 0x14),
                Port::new((base as u16) + 0x18),
                Port::new((base as u16) + 0x1c),
            ],
            current: 0usize,
            tppoll: Port::new((base as u16) + 0xd9),
        }
    }

    pub fn init(&mut self) {
        unsafe {
            self.config_1.write(0x0);
            self.cmd_reg.write(0x10);

            loop {
                if (self.cmd_reg.read() & 0x10) == 0 {
                    break;
                }
            }

            let ptr = VirtAddr::from_ptr(self.buffer.as_ptr());
            let physical = unsafe { translate_addr(ptr).unwrap() };
            println!("Sending VirtAddr: {:?} PhysAddr: {:?}", ptr, physical);
            self.rbstart.write(physical.as_u64() as u32);
            self.imr.write(0x809f);
//            self.imr.write(0x0005);
            self.wrap.write(0xf | (1 << 7));
            self.cmd_reg.write(0x0c);

            let data = [234u8; 120];
            self.write(&data);
            println!("Written");
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let ptr = VirtAddr::from_ptr(data.as_ptr());
        let physical = unsafe { translate_addr(ptr).unwrap() }.as_u64() as u32;

        unsafe {
            self.tx_dat[self.current].write(physical);
            self.tx_cmd[self.current].write((data.len() as u32) & 0xfff);

            loop {
                if (self.tx_cmd[self.current].read() & 0x8000) != 0 {
                    break;
                }
            }
        }

        self.current = (self.current + 1) % 4;

        // Force interrupt
        unsafe { self.tppoll.write(0xff) }
        let mut lel: Port<u32> = Port::new(0xc000 + 0x3e);
        unsafe {
            lel.write(0x1);
        }
    }
}

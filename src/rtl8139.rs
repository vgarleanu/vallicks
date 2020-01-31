use crate::prelude::*;
use alloc::boxed::Box;
use core::convert::TryInto;
use x86_64::instructions::port::Port;

pub struct RTL8139 {
    config_1: Port<u32>,
    cmd_reg: Port<u8>,
    rbstart: Port<u32>,
    imr: Port<u16>,
    wrap: Port<u32>,
    buffer: Box<[u8; 8192 + 16]>,
}

impl RTL8139 {
    pub fn new(base: u32) -> Self {
        Self {
            config_1: Port::new((base as u16) + 0x52),
            cmd_reg: Port::new((base as u16) + 0x37),
            rbstart: Port::new((base as u16) + 0x30),
            imr: Port::new((base as u16) + 0x3c),
            wrap: Port::new((base as u16) + 0x44),
            buffer: Box::new([0u8; 8192 + 16]),
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

            println!("Sending ptr: {:#x}", &self.buffer as *const _ as u32);
            self.rbstart.write(&self.buffer as *const _ as u32);
            self.imr.write(0x0005);
            self.wrap.write(0xf | (1 << 7));
            self.cmd_reg.write(0x0c);
        }
    }
}

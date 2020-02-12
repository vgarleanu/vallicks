use crate::arch::{interrupts::register_interrupt, memory::translate_addr, pci::Device};
use crate::net::ip::Ether2Frame;
use crate::prelude::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::convert::TryInto;
use spin::RwLock;
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

const RX_BUF_LEN: usize = 8192 + 16;

// Bit flags specific to the RCR
const APM: u32 = 1 << 1;
const AB: u32 = 1 << 3;
const MXDMA_UNLIMITED: u32 = 0b111 << 8;
const RXFTH_NONE: u32 = 0b111 << 13;

// Bit flags specific to the CR
const RST: u8 = 1 << 4;
const RX_ENABLE: u8 = 1 << 3;
const TX_ENABLE: u8 = 1 << 2;
const RX_BUF_EMPTY: u8 = 1 << 0;

// Bit flags specific to the C+CR
const RX_CHK_SUM: u16 = 1 << 5;
const CPRX: u16 = 1 << 1;
const CPTX: u16 = 1 << 0;

// Bit flags for IMR
const RX_OK: u16 = 1 << 0;
const RX_ERR: u16 = 1 << 1;
const TX_OK: u16 = 1 << 2;
const TX_ERR: u16 = 1 << 3;
const RDU: u16 = 1 << 4;
const TDU: u16 = 1 << 7;
const SYS_ERR: u16 = 1 << 15;

struct RTL8139Inner {
    pub config_1: Port<u32>,
    pub cmd_reg: Port<u8>,
    pub rbstart: Port<u32>,
    pub imr: Port<u16>,
    pub rcr: Port<u32>,

    pub buffer: Box<[u8; RX_BUF_LEN]>,
    pub rx_cursor: usize,

    pub tx_buffer: Box<[u8; 8192 + 16 + 1500]>,
    pub tx_dat: [Port<u32>; 4],
    pub tx_cmd: [Port<u32>; 4],
    pub current: usize,
    pub tppoll: Port<u8>,
    pub ack: Port<u16>,
    pub cpcr: Port<u16>,
    pub device: Device,
    pub capr: Port<u16>,

    pub rdsar_l: Port<u32>,
    pub rdsar_h: Port<u32>,

    pub tndps_l: Port<u32>,
    pub tndps_h: Port<u32>,

    frames: Vec<Ether2Frame>,
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
            rcr: Port::new(base + 0x44),

            buffer: Box::new([0u8; RX_BUF_LEN]),
            rx_cursor: 0,

            tx_buffer: Box::new([0u8; 8192 + 16 + 1500]),
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
            cpcr: Port::new(base + 0xe0),
            device,
            capr: Port::new(base + 0x38),

            rdsar_l: Port::new(base + 0xe4),
            rdsar_h: Port::new(base + 0xe8),

            tndps_l: Port::new(base + 0x20),
            tndps_h: Port::new(base + 0x24),

            frames: Vec::new(),
        };

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn init(&mut self) {
        let mut inner = self.inner.write();
        println!("rtl8139: Config start");
        // Turn the device on by writing to config_1 then reset the device to clear all data in the
        // buffers by writing 0x10 to cmd_reg
        unsafe {
            inner.config_1.write(0x0);
            inner.cmd_reg.write(RST);
        }

        // Wait while the device resets
        loop {
            if (unsafe { inner.cmd_reg.read() } & 0x10) == 0 {
                break;
            }
        }

        let rx_ptr = VirtAddr::from_ptr(inner.buffer.as_ptr());
        let rx_physical = unsafe { translate_addr(rx_ptr).unwrap() }.as_u64();

        let tx_ptr = VirtAddr::from_ptr(inner.tx_buffer.as_ptr());
        let tx_physical = unsafe { translate_addr(tx_ptr).unwrap() }.as_u64();

        println!(
            "rtl8139: Setting Rx buffer to VirtAddr: {:?} PhysAddr: {:#x?}",
            rx_ptr, rx_physical
        );

        println!(
            "rtl8139: Setting Tx buffer to VirtAddr: {:?} PhysAddr: {:#x?}",
            tx_ptr, tx_physical
        );

        // Unsafe block specific for pre-launch NIC config
        unsafe {
            // Accept Physically Match packets
            // Accept Broadcast packets
            // Enable Max DMA burst
            // No RX Threshold
            inner.rcr.write(APM | AB | MXDMA_UNLIMITED | RXFTH_NONE);

            // Enable Tx on the CR register
            inner.cmd_reg.write(RX_ENABLE | TX_ENABLE);

            /*
            // Enable C+ Mode
            inner.cpcr.write(RX_CHK_SUM | CPRX | CPTX);

            // Setup the Rx Ring buffer addrs for the NIC
            // NOTE:  tbf we dont really need to write high bits for the phys address
            inner.rdsar_l.write((rx_physical & 0xffffffff) as u32);
            inner
                .rdsar_h
                .write(((rx_physical >> 32) & 0xffffffff) as u32);

            // Setup the Tx Ring buffer addrs for the NIC
            inner.tndps_l.write((tx_physical & 0xffffffff) as u32);
            inner
                .tndps_h
                .write(((tx_physical >> 32) & 0xffffffff) as u32);
            */

            inner.rbstart.write(rx_physical as u32);
        }

        println!("rtl8139: Config done...");
        println!("rtl8139: Registering interrupt handler");

        // FIXME: Figure out a way to remove the double clone here
        let inner_clone = self.inner.clone();
        register_interrupt(43, move || Self::handle_int(inner_clone.clone()));
        println!("rtl8139: Registered interrupt handler");

        // Unsafe block specific to launch of NIC
        unsafe {
            // Enable Tx/Rx
            // NOTE: TX is technically already enabled but fuck it
            inner.cmd_reg.write(RX_ENABLE | TX_ENABLE);

            // Mask only RxOk, TxOk, and some Err registers for internal book-keeping
            inner
                .imr
                .write(RX_OK | TX_OK | RX_ERR | TX_ERR | SYS_ERR | RDU | TDU);
            inner.imr.write(0xffff);
        }
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

        if (isr & (1 << 0)) != 0 {
            inner.rok();
        }

        if (isr & (1 << 2)) != 0 {
            println!("rtl8139: TOK");
        }

        if (isr & (1 << 1)) != 0 {
            println!("rtl8139: RxErr");
        }

        if (isr & (1 << 3)) != 0 {
            println!("rtl8139: TxErr");
        }

        if (isr & (1 << 4)) != 0 {
            println!("rtl8139: Rx Buffer Overflow");
        }

        if (isr & (1 << 15)) != 0 {
            println!("rtl8139: SysErr");
        }

        println!("Reg: {:b}", isr);

        unsafe {
            inner.ack.write(isr);
        }
    }
}

impl RTL8139Inner {
    fn rok(&mut self) {
        let c = self.rx_cursor % RX_BUF_LEN;
        println!("Header: {:x?}", self.buffer[c..c + 4].to_vec());
        let length =
            u16::from_le_bytes(self.buffer[c + 2..c + 4].try_into().expect("Got wrong len"));

        if length < 1 {
            println!("Almost panicked :) idx: {}", self.rx_cursor);
            return;
        }

        let length = length - 4;
        println!("rtl8139: len {} {}", length, c);

        let frame = Ether2Frame::from_bytes(&self.buffer[c + 4..c + length as usize]);
        self.buffer[c+4] = 0xab;

        println!("{:?}", frame);
        self.frames.push(frame);

        self.rx_cursor += (length as usize + 4 + 3) & !3;
        if self.rx_cursor >= 3572 {
            self.rx_cursor = 0x10;
        }

        unsafe {
            println!("{}", self.rx_cursor);
            self.capr.write((self.rx_cursor - 0x10) as u16) // will reseting the offset to 0 fix?
        }

        if self.rx_cursor == 0x10 {
            self.rx_cursor = 0;
            return;
        }
        self.rx_cursor += 4;
    }
}

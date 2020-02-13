//! RTL8139 Network driver tested inside qemu.
//! Based on:
//! * http://www.jbox.dk/sanos/source/sys/dev/rtl8139.c.html
//! * https://www.cs.usfca.edu/~cruse/cs326f04/RTL8139_ProgrammersGuide.pdf
//! * https://www.cs.usfca.edu/~cruse/cs326f04/RTL8139D_DataSheet.pdf

use crate::arch::{interrupts::register_interrupt, memory::translate_addr, pci::Device};
use crate::net::ip::Ether2Frame;
use crate::prelude::*;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::convert::TryInto;
use spin::RwLock;
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

// Here we define all the RX Buffer lengths that we are gonna use
// NOTE: Make sure you do all your logic and math with RX_BUF_LEN as technically
//       the padding bytes and the Wrap bytes shouldnt exist, seriously it took me a long long long
//       time to figure out that you should do all the math for calculating the new cursor without
//       also adding the padding length.
const RX_BUF_LEN: usize = 8192;
const RX_BUF_WRAP: usize = 1500; // Extra 1500 bytes with a WRAP mask for Rx because i really cant be fucked
const RX_BUF_PAD: usize = 16;
const RX_BUF_LEN_WRAPPED: usize = RX_BUF_LEN + RX_BUF_PAD + RX_BUF_WRAP;

// Bit flags specific to the RCR
const APM: u32 = 0b10;
const AB: u32 = 0b1000;
const WRAP: u32 = 0b1000_0000;
const MXDMA_UNLIMITED: u32 = 0b111_0000_0000;
const RXFTH_NONE: u32 = 0b1110_0000_0000_0000;

// Bit flags specific to the CR
#[allow(dead_code)]
const RX_BUF_EMPTY: u8 = 0b1;
const TX_ENABLE: u8 = 0b100;
const RX_ENABLE: u8 = 0b1000;
const RST: u8 = 0b10000;

// Bit flags for IMR
const RX_OK: u16 = 0b1;
const RX_ERR: u16 = 0b10;
const TX_OK: u16 = 0b100;
const TX_ERR: u16 = 0b1000;
const RDU: u16 = 0b10000;
const TDU: u16 = 0b1000_0000;
const SYS_ERR: u16 = 0b1000_0000_0000_0000;

/// This is our inner struct that holds all of our ports that we will use to talk with the nic, as
/// well as our rx and tx buffers.
struct RTL8139Inner {
    pub device: Device,
    pub config_1: Port<u32>,
    pub cmd_reg: Port<u8>,
    pub rbstart: Port<u32>,
    pub imr: Port<u16>,
    pub rcr: Port<u32>,
    pub tppoll: Port<u8>,
    pub ack: Port<u16>,
    pub cpcr: Port<u16>,
    pub capr: Port<u16>,

    pub tx_dat: [Port<u32>; 4],
    pub tx_cmd: [Port<u32>; 4],
    pub tx_cursor: usize,

    pub buffer: [u8; RX_BUF_LEN_WRAPPED],
    pub rx_cursor: usize,

    frames: Vec<Ether2Frame>,
}

pub struct RTL8139 {
    inner: Arc<RwLock<RTL8139Inner>>,
}

impl RTL8139 {
    pub fn new(device: Device) -> Self {
        let base = device.port_base.unwrap() as u16;
        let inner = RTL8139Inner {
            device,
            config_1: Port::new(base + 0x52),
            cmd_reg: Port::new(base + 0x37),
            rbstart: Port::new(base + 0x30),
            imr: Port::new(base + 0x3c),
            rcr: Port::new(base + 0x44),
            tppoll: Port::new(base + 0xd9),
            ack: Port::new(base + 0x3e),
            cpcr: Port::new(base + 0xe0),
            capr: Port::new(base + 0x38),

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
            tx_cursor: 0,

            buffer: [0u8; RX_BUF_LEN_WRAPPED],
            rx_cursor: 0,

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

        println!(
            "rtl8139: Setting Rx buffer to VirtAddr: {:?} PhysAddr: {:#x?}",
            rx_ptr, rx_physical
        );

        // Unsafe block specific for pre-launch NIC config
        unsafe {
            // Accept Physically Match packets
            // Accept Broadcast packets
            // Enable Max DMA burst
            // No RX Threshold
            inner
                .rcr
                .write(APM | AB | MXDMA_UNLIMITED | RXFTH_NONE | WRAP);

            // Enable Tx on the CR register
            inner.cmd_reg.write(RX_ENABLE | TX_ENABLE);

            // Write the PHYSICAL address of our Rx buffer to the NIC
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
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let mut inner = self.inner.write();

        // Disable interrupts for this PCI device to avoid deadlock to do with inner
        inner.device.set_disable_int();

        {
            let cursor = inner.tx_cursor;
            let ptr = VirtAddr::from_ptr(data.as_ptr());
            let physical = unsafe { translate_addr(ptr).unwrap() }.as_u64() as u32;

            unsafe {
                inner.tx_dat[cursor].write(physical);
                inner.tx_cmd[cursor].write((data.len() as u32) & 0xfff);

                loop {
                    if (inner.tx_cmd[cursor].read() & 0x8000) != 0 {
                        break;
                    }
                }
            }
            inner.tx_cursor = (cursor + 1) % 4;
        }

        inner.device.set_enable_int();
    }

    fn handle_int(inner: Arc<RwLock<RTL8139Inner>>) {
        let mut inner = inner.write();
        let isr = unsafe { inner.ack.read() };

        if (isr & RX_OK) != 0 {
            inner.rok();
        }

        if (isr & TX_OK) != 0 {
            println!("rtl8139: TOK");
        }

        if (isr & RX_ERR) != 0 {
            println!("rtl8139: RxErr");
        }

        if (isr & TX_ERR) != 0 {
            println!("rtl8139: TxErr");
        }

        if (isr & (RDU | TDU)) != 0 {
            println!("rtl8139: Rx/Tx Buffer Overflow");
        }

        if (isr & SYS_ERR) != 0 {
            println!("rtl8139: SysErr");
        }

        unsafe {
            inner.ack.write(isr);
        }
    }
}

impl RTL8139Inner {
    /// Function called on a ROK interrupt from the RTL8139 NIC, it parses the data written into
    /// the buffer as a ethernet frame and pushes it into our Vec.
    fn rok(&mut self) {
        // A packet frame looks something like this
        // +--------------------------------------------+
        // | |     HEADER     |            |   DATA   | |
        // | +----------------+            +----------+ |
        // | |??|len = 2 bytes| = 4 bytes  |data = len| |
        // | +----------------+            +----------+ |
        // +--------------------------------------------+
        //
        // As per the diagram the packet structure is a 4 byte header where the last 2 bytes is the
        // length of the incoming data.
        // The length given also includes the length of the header itself.
        let buffer = &self.buffer[self.rx_cursor..];
        let length = u16::from_le_bytes(buffer[2..4].try_into().expect("Got wrong len")) as usize;

        // NOTE: The length in the header will never be less than 64, if a packet is received that
        //       has a length less than 64, the NIC will simply pad the packet with 0x00.
        assert!(length >= 64);

        // NOTE: We are currently not zeroing out memory after a packet has been parsed and pushed.
        //       Are we sure that if packets with length less than 64 bytes will not contain
        //       remnants of the old packets?
        let frame = Ether2Frame::from_bytes(&self.buffer[4..length - 4]);
        self.frames.push(frame);

        // Here we set the new index/cursor from where to read new packets, self.rx_cursor should
        // always point to the start of the header.
        // To calculate the new cursor we add the length of the previous frame which SHOULD include
        // the 4 bytes for the header, we also add 3 for 32 bit alignment and then mask the result.
        self.rx_cursor = (self.rx_cursor + length as usize + 4 + 3) & !3;

        unsafe {
            // The NIC is then informed of the new cursor. We remove 0x10 to avoid a overflow as
            // the NIC takes the padding into account I think.
            self.capr.write((self.rx_cursor - 0x10) as u16);
        }

        self.rx_cursor = self.rx_cursor % RX_BUF_LEN;
    }
}

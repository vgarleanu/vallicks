//! RTL8139 Network driver tested inside qemu.
//! Based on:
//! * http://www.jbox.dk/sanos/source/sys/dev/rtl8139.c.html
//! * https://www.cs.usfca.edu/~cruse/cs326f04/RTL8139_ProgrammersGuide.pdf
//! * https://www.cs.usfca.edu/~cruse/cs326f04/RTL8139D_DataSheet.pdf
use crate::{
    arch::{interrupts::register_interrupt, memory::translate_addr, pci::Device},
    net::frames::{eth2::Ether2Frame, mac::Mac},
    prelude::sync::{Arc, RwLock},
    prelude::*,
};
use conquer_once::spin::OnceCell;

use core::convert::TryInto;
use crossbeam_queue::SegQueue;

use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::sink::Sink;
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;
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

static FRAMES: OnceCell<SegQueue<Ether2Frame>> = OnceCell::uninit();
static RTL8139_STATE: OnceCell<Arc<RwLock<Rtl8139State>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// This is our inner struct that holds all of our ports that we will use to talk with the nic, as
/// well as our rx and tx buffers.
struct Rtl8139State {
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

    // Registers holding our MAC bytes
    pub idr: [Port<u8>; 6],

    pub tx_dat: [Port<u32>; 4],
    pub tx_cmd: [Port<u32>; 4],
    pub tx_cursor: usize,

    pub buffer: [u8; RX_BUF_LEN_WRAPPED],
    pub rx_cursor: usize,
}

pub struct RTL8139 {
    mac: Mac,
}

impl RTL8139 {
    pub fn new(device: Device) -> Self {
        let base = device.port_base.unwrap() as u16;
        let inner = Rtl8139State {
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

            idr: [
                Port::new(base + 0x00),
                Port::new(base + 0x01),
                Port::new(base + 0x02),
                Port::new(base + 0x03),
                Port::new(base + 0x04),
                Port::new(base + 0x05),
            ],

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
        };

        FRAMES.init_once(|| SegQueue::new());
        RTL8139_STATE.init_once(move || Arc::new(RwLock::new(inner)));

        Self {
            mac: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff].into(),
        }
    }

    pub fn init(&mut self) {
        let mut inner = RTL8139_STATE.try_get().unwrap().write();
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

        let raw_mac = inner
            .idr
            .iter_mut()
            .map(|x| unsafe { x.read() })
            .collect::<Vec<u8>>();
        self.mac = raw_mac.as_slice().into();
        println!("rtl8139: Got MAC {}", self.mac);

        let rx_ptr = VirtAddr::from_ptr(inner.buffer.as_ptr());
        let rx_physical = unsafe {
            translate_addr(rx_ptr)
                .expect("rtl8139: Failed to translate RxPtr from VirtAddr to PhysAddr")
        }
        .as_u64();

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
                //                .write(APM | AB | MXDMA_UNLIMITED | RXFTH_NONE | WRAP);
                .write(0xffffffff);

            // Enable Tx on the CR register
            inner.cmd_reg.write(RX_ENABLE | TX_ENABLE);

            // Write the PHYSICAL address of our Rx buffer to the NIC
            inner.rbstart.write(rx_physical as u32);
        }

        println!("rtl8139: Config done...");
        println!("rtl8139: Registering interrupt handler");

        // FIXME: Figure out a way to remove the double clone here
        register_interrupt(43, Self::handle_int);
        println!("rtl8139: Registered interrupt handler");

        // Unsafe block specific to launch of NIC
        unsafe {
            // Enable Tx/Rx
            // NOTE: TX is technically already enabled but fuck it
            inner.cmd_reg.write(RX_ENABLE | TX_ENABLE);

            // Mask only RxOk, TxOk, and some Err registers for internal book-keeping
            inner
                .imr
                .write(0xffff | RX_OK | TX_OK | RX_ERR | TX_ERR | SYS_ERR | RDU | TDU);
        }
    }

    pub fn mac(&self) -> Mac {
        self.mac.clone()
    }

    pub fn try_read(&mut self) -> Option<Ether2Frame> {
        if FRAMES
            .try_get()
            .expect("rtl8139: frame queue uninit")
            .is_empty()
        {
            return None;
        }

        FRAMES.try_get().expect("rtl8139: frame queue uninit").pop()
    }

    pub fn split(&mut self) -> (RxSink, TxSink) {
        (RxSink::new(), TxSink::new())
    }

    extern "x86-interrupt" fn handle_int(_: &mut InterruptStackFrame) {
        let mut inner = RTL8139_STATE.try_get().unwrap().write();
        let isr = unsafe { inner.ack.read() };

        if (isr & RX_OK) != 0 {
            while (unsafe { inner.cmd_reg.read() } & RX_BUF_EMPTY) == 0 {
                if let Some(x) = inner.rok() {
                    FRAMES.try_get().unwrap().push(x);
                    WAKER.wake();
                }
            }
            println!("rtl8139: ROK");
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

        crate::arch::interrupts::notify_eoi(43);
    }
}

pub struct RxSink {
    _private: (),
}

impl RxSink {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl Stream for RxSink {
    type Item = Ether2Frame;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(x) = FRAMES.try_get().unwrap().pop() {
            return Poll::Ready(Some(x));
        }

        WAKER.register(&cx.waker());

        match FRAMES.try_get().unwrap().pop() {
            Some(x) => {
                WAKER.take();
                Poll::Ready(Some(x))
            }
            None => Poll::Pending,
        }
    }
}

pub struct TxSink<'a> {
    buffer: Vec<&'a [u8]>,
    netdev: Device,
}

impl<'a> TxSink<'a> {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            netdev: RTL8139_STATE.try_get().unwrap().read().device.clone(),
        }
    }
}

impl<'a> Sink<&'a [u8]> for TxSink<'a> {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: &'a [u8]) -> Result<(), Self::Error> {
        self.buffer.push(item);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.netdev.set_disable_int();
        {
            let mut lock = if let Some(x) = RTL8139_STATE
                .try_get()
                .expect("rtl8139: state not init")
                .try_write()
            {
                x
            } else {
                return Poll::Pending;
            };

            for item in self.buffer.drain(..) {
                unsafe { lock.write(item) };
            }
        }
        self.netdev.set_enable_int();

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}

impl Rtl8139State {
    /// Function called on a ROK interrupt from the RTL8139 NIC, it parses the data written into
    /// the buffer as a ethernet frame and pushes it into our Vec.
    fn rok(&mut self) -> Option<Ether2Frame> {
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
        assert!(
            length >= 64,
            "rtl8139: ROK Len is less than 64. THIS IS A BUG."
        );

        // NOTE: We are currently not zeroing out memory after a packet has been parsed and pushed.
        //       Are we sure that if packets with length less than 64 bytes will not contain
        //       remnants of the old packets?
        // If the frame is correctly parsed we push it into the queue, otherwise just skip it
        let frame = buffer[4..length - 4].try_into().ok();

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

        frame
    }

    /// # Safety
    /// The caller must make sure that interrupts are disabled before calling and are re-enabled
    /// after calling or the program will deadlock.
    pub unsafe fn write(&mut self, data: &[u8]) {
        // NOTE: Are we sure we absolutely need to disable interrupts? maybe we can bypass this
        //       with DMA.
        // We clone the inner PCI device to avoid a deadlock when we re-enable PCI interrupts for
        // this device

        // Disable interrupts for this PCI device to avoid deadlock to do with inner
        let cursor = self.tx_cursor;
        let ptr = VirtAddr::from_ptr(data.as_ptr());
        let physical = translate_addr(ptr).unwrap().as_u64() as u32;

        self.tx_dat[cursor].write(physical);
        self.tx_cmd[cursor].write((data.len() as u32) & 0xfff);

        loop {
            if (self.tx_cmd[cursor].read() & 0x8000) != 0 {
                break;
            }
        }

        self.tx_cursor = (cursor + 1) % 4;
    }
}

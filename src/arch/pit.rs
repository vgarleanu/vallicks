#![allow(unused_variables)]
#![allow(unused_imports)]
use crate::prelude::*;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;

const PIT_CH0: u16 = 0x40; // Channel 0 data port (read/write) (PIC TIMER)
const PIT_REG: u16 = 0x43; // Mode/Command register (write only, a read is ignored)

// set frequency approximately to 1000 Hz
// frequency = (1193182 / reload) Hz <=> reload = (1193182 Hz / frequency)
// reload = 1193182.0 / 1000.0 = 1193.182 => round => 1193
// actual_freq = 1193182.0/reload = 1000.15255660
// time_between = 1/actual_freq = 0.0009998474667 s ~= 999.847467 ns
// floats are disabled in kernel code, so these are calculated by hand
#[allow(dead_code)]
const TARGET_FREQ: u64 = 150; // Hz
const RELOAD_VALUE: u64 = 1193;

pub const ACTUAL_FREQ_E_9: u64 = 1000_152556600; // Hz * 10 ** 12
pub const TIME_BETWEEN_E_12: u64 = 999847467; // s * 10 ** 12
pub const TIME_BETWEEN_E_6: u64 = 999;

lazy_static::lazy_static! {
    static ref P_PIT1_CH0: Mutex<Port<u8>> = Mutex::new(Port::new(PIT_CH0));
    static ref TICK: AtomicU64 = AtomicU64::new(0);
}

pub fn init() {
    // Channel 0, lobyte/hibyte, Rate generator, Binary mode
    let mut p_pit1_ch0 = P_PIT1_CH0.lock();
    let mut p_pit1_reg = Port::new(PIT_REG);
    unsafe {
        //p_pit1_reg.write(0b00_11_000_0 as u32); // command
        p_pit1_reg.write(0b00_11_011_0 as u32);
        p_pit1_ch0.write((RELOAD_VALUE & 0x00FF) as u8); // low
        p_pit1_ch0.write(((RELOAD_VALUE & 0xFF00) >> 8) as u8); // high
    }

    println!("[PIT] PIT Setup done...");
}

pub fn tick() {
    TICK.fetch_add(1, Ordering::SeqCst);
    if TICK.load(Ordering::SeqCst) % 1000 == 0 {
        get_secs();
    }
}

pub fn get_secs() -> u64 {
    TICK.load(Ordering::SeqCst) / TIME_BETWEEN_E_6
}

pub fn get_milis() -> u64 {
    TICK.load(Ordering::SeqCst)
}

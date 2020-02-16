use crate::prelude::{ *};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

const PIT_CH0: u16 = 0x40; // Channel 0 data port (read/write) (PIC TIMER)
const PIT_REG: u16 = 0x43; // Mode/Command register (write only, a read is ignored)

const TARGET_FREQ: u64 = 5000; // Hz
const RELOAD_VALUE: u64 = 1193182 / TARGET_FREQ;

lazy_static::lazy_static! {
    static ref TICK: AtomicU64 = AtomicU64::new(1);
}

pub fn init() {
    let mut p_pit1_ch0 = Port::new(PIT_CH0);
    let mut p_pit1_reg = Port::new(PIT_REG);
    unsafe {
        p_pit1_reg.write(0x36u8);
        p_pit1_ch0.write((RELOAD_VALUE & 0x00FF) as u8); // low
        p_pit1_ch0.write(((RELOAD_VALUE & 0xFF00) >> 8) as u8); // high
    }

    println!("pit: PIT Setup done...");
}

pub fn tick() {
    TICK.fetch_add(1, Ordering::SeqCst);
    if TICK.load(Ordering::SeqCst) % 1000 == 0 {
        get_secs();
    }
}

pub fn get_secs() -> u64 {
    TICK.load(Ordering::SeqCst) / TARGET_FREQ
}

pub fn get_milis() -> u64 {
    TICK.load(Ordering::SeqCst)
}

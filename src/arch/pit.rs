//! This module holds common functions needed to set up the CPU Timer interrupt and handle each
//! tick.
use crate::prelude::*;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

const PIT_CH0: u16 = 0x40; // Channel 0 data port (read/write) (PIC TIMER)
const PIT_REG: u16 = 0x43; // Mode/Command register (write only, a read is ignored)

/// Represents the value we send to the PIT to set up the desired frequency
const RELOAD_VALUE: u64 = 1_193_182;
/// Tick divider for milis.
static mut TICK_FREQ: u64 = 18; //default of 18hz or every 55ms
/// Total ticks since boot.
static TICK: AtomicU64 = AtomicU64::new(0);
/// Timeslot set by the async executor representing when we need to wake some tasks that are
/// awaiting a timer.
static NOTIFY_AT: AtomicU64 = AtomicU64::new(0); // default to never

/// Function sets the PIT up with the desired frequency.
pub fn init(freq: u16) {
    let freq = freq.max(18) as u64;
    let reload_value = (RELOAD_VALUE / freq).min(u16::MAX as u64) as u16;

    println!("pit: freq={} reload_value={}", freq, reload_value);

    let mut p_pit1_ch0 = Port::new(PIT_CH0);
    let mut p_pit1_reg = Port::new(PIT_REG);

    unsafe {
        TICK_FREQ = freq;
        p_pit1_reg.write(0x36u8);
        p_pit1_ch0.write((reload_value & 0x00FF) as u8); // low
        p_pit1_ch0.write(((reload_value & 0xFF00) >> 8) as u8); // high
    }

    println!("pit: PIT Setup done...");
}

/// Function is called when a timer interrupt occurs and increments the inner count of tick so far
pub fn tick() {
    TICK.fetch_add(1, Ordering::SeqCst);
    let notify_at = NOTIFY_AT.load(Ordering::SeqCst);

    // if we have passed the deadline or are exactly at it
    if notify_at != 0 && notify_at <= get_milis() {
        crate::async_::wake_tasks();
    }
}

/// Converts the ticks into seconds since boot
pub fn get_secs() -> u64 {
    unsafe { TICK.load(Ordering::SeqCst) / TICK_FREQ }
}

/// Converts tick into miliseconds since boot
pub fn get_milis() -> u64 {
    unsafe { TICK.load(Ordering::SeqCst) / TICK_FREQ * 1000 }
}

/// return ticks
pub fn ticks() -> u64 {
    TICK.load(Ordering::SeqCst)
}

/// allows us to wake up any async tasks that depend on a timer future.
pub(crate) fn notify_in(milis: u128) {
    NOTIFY_AT.store(milis as u64, Ordering::Relaxed);
}

/// Reset the time slot.
pub(crate) fn reset_notify() {
    NOTIFY_AT.store(0, Ordering::Relaxed);
}

#![allow(missing_docs)] // NOTE: document this later
use crate::arch::pit::get_milis;
use crate::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));
// We are gonna fake the current EPOCH because we havent implemented any real clock yet
pub const FAKE_EPOCH: u64 = 1582162024; // 20/02/2019 01:28 AM

fn current_time() -> Duration {
    Duration::from_millis(FAKE_EPOCH + (get_milis() / 1000))
}

impl Instant {
    pub fn now() -> Instant {
        Instant(current_time())
    }

    pub const fn zero() -> Instant {
        Instant(Duration::from_secs(0))
    }

    pub fn actually_monotonic() -> bool {
        true
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

impl SystemTime {
    pub fn now() -> SystemTime {
        SystemTime(current_time())
    }

    pub fn from_wasi_timestamp(ts: u64) -> SystemTime {
        SystemTime(Duration::from_nanos(ts))
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}

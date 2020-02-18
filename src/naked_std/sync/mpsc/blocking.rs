//! Generic support for building blocking abstractions.

// TODO: Create a proper thread interface
use crate::naked_std::thread::{self, ThreadId as Thread};
use alloc::sync::Arc;
use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

struct Inner {
    thread: Thread,
    woken: AtomicBool,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

#[derive(Clone)]
pub struct SignalToken {
    inner: Arc<Inner>,
}

pub struct WaitToken {
    inner: Arc<Inner>,
}

impl !Send for WaitToken {}

impl !Sync for WaitToken {}

pub fn tokens() -> (WaitToken, SignalToken) {
    let inner = Arc::new(Inner {
        thread: thread::current(),
        woken: AtomicBool::new(false),
    });
    let wait_token = WaitToken {
        inner: inner.clone(),
    };
    let signal_token = SignalToken { inner };
    (wait_token, signal_token)
}

impl SignalToken {
    pub fn signal(&self) -> bool {
        let wake = !self
            .inner
            .woken
            .compare_and_swap(false, true, Ordering::SeqCst);
        /* NOTE: Implement thread parking
        if wake {
            self.inner.thread.unpark();
        }
        */
        wake
    }

    /// Converts to an unsafe usize value. Useful for storing in a pipe's state
    /// flag.
    #[inline]
    pub unsafe fn cast_to_usize(self) -> usize {
        mem::transmute(self.inner)
    }

    /// Converts from an unsafe usize value. Useful for retrieving a pipe's state
    /// flag.
    #[inline]
    pub unsafe fn cast_from_usize(signal_ptr: usize) -> SignalToken {
        SignalToken {
            inner: mem::transmute(signal_ptr),
        }
    }
}

impl WaitToken {
    pub fn wait(self) {
        while !self.inner.woken.load(Ordering::SeqCst) {
            // NOTE: We might want to actually make a park function to limit high cpu usage, but
            //       this should do for now
            thread::yield_now()
        }
    }
}

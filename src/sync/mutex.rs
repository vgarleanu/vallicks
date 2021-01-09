use core::cell::UnsafeCell;
use core::future::Future;
use core::ops::Deref;
use core::ops::DerefMut;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::Acquire;
use core::sync::atomic::Ordering::Relaxed;
use core::sync::atomic::Ordering::Release;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

use crate::prelude::*;
use crate::sync::waker_set::WakerSet;

pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
    wakers: WakerSet,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(value),
            wakers: WakerSet::new(),
        }
    }
}

impl<T> Mutex<T> {
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        struct LockFuture<'a, T> {
            lock: &'a Mutex<T>,
        }

        impl<'a, T> Future for LockFuture<'a, T> {
            type Output = MutexGuard<'a, T>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.lock.try_lock() {
                    Some(guard) => return Poll::Ready(guard),
                    None => {
                        self.lock.wakers.insert(cx);
                        return Poll::Pending;
                    }
                }
            }
        }

        LockFuture { lock: self }.await
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if let Ok(false) = self
            .lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
        {
            return Some(MutexGuard {
                lock: &self.lock,
                value: unsafe { &mut *self.data.get() },
                wakers: &self.wakers,

            });
        }
        None
    }

    pub fn register_waker(&self, cx: &Context<'_>) {
        self.wakers.insert(cx);
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send + Sync> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    value: &'a mut T,
    wakers: &'a WakerSet,
}

impl<'a, T: ?Sized + 'a> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<'a, T: ?Sized + 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Release);
        self.wakers.notify_one();
    }
}

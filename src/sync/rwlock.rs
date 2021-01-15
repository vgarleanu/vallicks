use core::cell::UnsafeCell;
use core::fmt;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Acquire;
use core::sync::atomic::Ordering::Relaxed;
use core::sync::atomic::Ordering::SeqCst;
use crate::sync::waker_set::WakerSet;
use core::task::{Context, Poll};

/// Set if a write lock is held.
const WRITE_LOCK: u64 = 1 << 0;

/// The value of a single blocked read contributing to the read count.
const ONE_READ: u64 = 1 << 1;

/// The bits in which the read count is stored.
const READ_COUNT_MASK: u64 = !(ONE_READ - 1);

pub struct RwLock<T: ?Sized> {
    state: AtomicU64,
    read_wakers: WakerSet,
    write_wakers: WakerSet,
    value: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(t: T) -> RwLock<T> {
        RwLock {
            state: AtomicU64::new(0),
            read_wakers: WakerSet::new(),
            write_wakers: WakerSet::new(),
            value: UnsafeCell::new(t),
        }
    }
}

impl<T> RwLock<T> {
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        pub struct ReadFuture<'a, T> {
            lock: &'a RwLock<T>,
        }

        impl<'a, T> Future for ReadFuture<'a, T> {
            type Output = RwLockReadGuard<'a, T>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.lock.try_read() {
                    Some(guard) => return Poll::Ready(guard),
                    None => {
                        self.lock.read_wakers.insert(cx);
                        return Poll::Pending;
                    }
                }
            }
        }

        ReadFuture {
            lock: self,
        }
        .await
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let state = self.state.load(SeqCst);

        // If a write lock is currently held, then a read lock cannot be acquired.
        if state & WRITE_LOCK != 0 {
            return None;
        }

        // Make sure the number of readers doesn't overflow.
        if state > i64::MAX as u64 {
            panic!("Overflowed max readers");
        }

        if let Ok(_) = self.state.compare_exchange_weak(state, state + ONE_READ, Acquire, Relaxed) {
            return Some(RwLockReadGuard(self));
        }

        None
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        pub struct WriteFuture<'a, T> {
            lock: &'a RwLock<T>,
            opt_key: Option<usize>,
        }

        impl<'a, T> Future for WriteFuture<'a, T> {
            type Output = RwLockWriteGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.lock.try_write() {
                    Some(guard) => return Poll::Ready(guard),
                    None => {
                        // Insert this lock operation.
                        self.opt_key = Some(self.lock.write_wakers.insert(cx));
                        return Poll::Pending;
                    }
                }
            }
        }

        impl<T> Drop for WriteFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
                if let Some(key) = self.opt_key {
                    self.lock.write_wakers.cancel(key);
                }
            }
        }

        WriteFuture {
            lock: self,
            opt_key: None,
        }
        .await
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if let Ok(0) = self.state.compare_exchange(0, WRITE_LOCK, Acquire, Relaxed) {
            Some(RwLockWriteGuard(self))
        } else {
            None
        }
    }
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T: fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Locked;
        impl fmt::Debug for Locked {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<locked>")
            }
        }

        match self.try_read() {
            None => f.debug_struct("RwLock").field("data", &Locked).finish(),
            Some(guard) => f.debug_struct("RwLock").field("data", &&*guard).finish(),
        }
    }
}

/// A guard that releases the read lock when dropped.
pub struct RwLockReadGuard<'a, T>(&'a RwLock<T>);

unsafe impl<T: Send> Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let state = self.0.state.fetch_sub(ONE_READ, SeqCst);

        // If this was the last reader, notify a blocked writer if none were notified already.
        if state & READ_COUNT_MASK == ONE_READ {
            self.0.write_wakers.notify_any();
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

/// A guard that releases the write lock when dropped.
pub struct RwLockWriteGuard<'a, T>(&'a RwLock<T>);

unsafe impl<T: Send> Send for RwLockWriteGuard<'_, T> {}
unsafe impl<T: Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.0.state.store(0, SeqCst);

        // Notify all blocked readers.
        if !self.0.read_wakers.notify_all() {
            // If there were no blocked readers, notify a blocked writer if none were notified
            // already.
            self.0.write_wakers.notify_any();
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}

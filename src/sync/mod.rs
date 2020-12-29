pub mod mpsc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use crossbeam_queue::SegQueue;
use futures_util::task::AtomicWaker;
use core::task::Context;
use core::task::Poll;

pub(crate) fn channel<T, S: Semaphore>(semaphore: S) -> (Tx<T, S>, Rx<T, S>) {
    let list = Arc::new(SegQueue::new());

    let chan = Arc::new(Chan {
        tx: list.clone(),
        semaphore,
        rx_waker: AtomicWaker::new(),
        tx_count: AtomicUsize::new(1),
        rx_fields: UnsafeCell::new(RxFields {
            list,
            rx_closed: false,
        })
    });

    (Tx::new(chan.clone()), Rx::new(chan.clone()))
}

pub trait Semaphore {
    fn add_permit(&self);
    fn is_idle(&self) -> bool;
    fn close(&self);
    fn is_closed(&self) -> bool;
}

impl Semaphore for AtomicUsize {
    fn add_permit(&self) {
        let prev = self.fetch_sub(2, Ordering::Release);

        if prev >> 1 == 0 {
            panic!("unexpected atomic value: {}", prev >> 1);
        }
    }

    fn is_idle(&self) -> bool {
        self.load(Ordering::Acquire) >> 1 == 0
    }

    fn close(&self) {
        self.fetch_or(1, Ordering::Release);
    }

    fn is_closed(&self) -> bool {
        self.load(Ordering::Acquire) & 1 == 1
    }
}

struct RxFields<T> {
    list: Arc<SegQueue<T>>,
    rx_closed: bool,
}

struct Chan<T, S> {
    tx: Arc<SegQueue<T>>,
    semaphore: S,
    rx_waker: AtomicWaker,
    tx_count: AtomicUsize,
    rx_fields: UnsafeCell<RxFields<T>>,
}

impl<T, S> Chan<T, S> {
    fn send(&self, value: T) {
        self.tx.push(value);
        self.rx_waker.wake();
    }
}

unsafe impl<T: Send, S: Send> Send for Chan<T, S> {}
unsafe impl<T: Sync, S: Sync> Sync for Chan<T, S> {}

pub(crate) struct Tx<T, S> {
    inner: Arc<Chan<T, S>>,
}

impl<T, S> Tx<T, S> {
    fn new(chan: Arc<Chan<T, S>>) -> Self {
        Self { inner: chan }
    }

    pub(super) fn semaphore(&self) -> &S {
        &self.inner.semaphore
    }

    pub(crate) fn send(&self, value: T) {
        self.inner.send(value);
    }

    pub(crate) fn wake_rx(&self) {
        self.inner.rx_waker.wake();
    }
}

impl<T, S: Semaphore> Tx<T, S> {
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.semaphore.is_closed()
    }

    pub(crate) async fn closed(&self) {
        self.inner.semaphore.close();
    }
}

impl<T, S> Clone for Tx<T, S> {
    fn clone(&self) -> Self {
        self.inner.tx_count.fetch_add(1, Ordering::Relaxed);

        Tx {
            inner: self.inner.clone()
        }
    }
}

impl<T, S> Drop for Tx<T, S> {
    fn drop(&mut self) {
        if self.inner.tx_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        self.wake_rx();
    }
}

pub(crate) struct Rx<T, S> {
    inner: Arc<Chan<T, S>>,
}

impl<T, S: Semaphore> Rx<T, S> {
    fn new(chan: Arc<Chan<T, S>>) -> Self {
        Self { inner: chan }
    }

    pub(crate) fn close(&mut self) {
        let rx_fields = unsafe { &mut *Arc::get_mut_unchecked(&mut self.inner).rx_fields.get_mut() };

        if !rx_fields.rx_closed {
            rx_fields.rx_closed = true;
        }
        self.inner.semaphore.close();
    }

    pub(crate) fn recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        macro_rules! try_recv {
            () => {
                let rx_fields = unsafe { &mut *Arc::get_mut_unchecked(&mut self.inner).rx_fields.get_mut() };
                match rx_fields.list.pop() {
                    Some(value) => {
                        self.inner.semaphore.add_permit();
                        return Poll::Ready(Some(value));
                    }
                    None => {} // fall through
                }
            };
        }

        try_recv!();

        self.inner.rx_waker.register(cx.waker());

        try_recv!();

        if self.inner.semaphore.is_idle() {
            let rx_fields = unsafe { &mut *Arc::get_mut_unchecked(&mut self.inner).rx_fields.get_mut() };
            if rx_fields.rx_closed {
                return Poll::Ready(None)
            }
        }

        Poll::Pending
    }
}

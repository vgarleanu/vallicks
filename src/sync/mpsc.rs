use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use futures_util::future::poll_fn;

pub fn channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (tx, rx) = super::channel(AtomicUsize::new(0));

    let tx = UnboundedSender::new(tx);
    let rx = UnboundedReceiver::new(rx);

    (tx, rx)
}

pub struct UnboundedSender<T> {
    chan: super::Tx<T, AtomicUsize>,
}

impl<T> UnboundedSender<T> {
    pub(crate) fn new(chan: super::Tx<T, AtomicUsize>) -> Self {
        Self { chan }
    }

    pub fn send(&self, message: T) -> Result<(), ()> {
        if !self.inc_num_messages() {
            return Err(());
        }

        self.chan.send(message);
        self.chan.wake_rx();
        Ok(())
    }

    fn inc_num_messages(&self) -> bool {
        let mut curr = self.chan.semaphore().load(Ordering::Acquire);

        loop {
            if curr & 1 == 1 {
                return false
            }

            if curr == usize::MAX ^ 1 {
                panic!("overflowed ref count");
            }

            match self.chan.semaphore().compare_exchange(curr, curr + 2, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return true,
                Err(e) => { curr = e },
            }
        }
    }
    
    pub async fn closed(&self) {
        self.chan.closed().await
    }

    pub fn is_closed(&self) -> bool {
        self.chan.is_closed()
    }
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            chan: self.chan.clone(),
        }
    }
}

pub struct UnboundedReceiver<T> {
    chan: super::Rx<T, AtomicUsize>,
}

impl<T> UnboundedReceiver<T> {
    pub(crate) fn new(chan: super::Rx<T, AtomicUsize>) -> Self {
        Self { chan }
    }

    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn close(&mut self) {
        self.chan.close();
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.chan.recv(cx)
    }
}

unsafe impl<T> Sync for UnboundedReceiver<T> {}

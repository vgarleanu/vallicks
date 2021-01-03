pub mod executor;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use core::time::Duration;

use crossbeam_queue::SegQueue;
use futures_util::task::Waker;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use super::arch::pit::get_milis;
use super::prelude::*;

lazy_static::lazy_static! {
    pub static ref SPAWN_QUEUE: Arc<SegQueue<Task>> = Arc::new(SegQueue::new());
    static ref TIMER_QUEUE: Arc<Mutex<BTreeMap<Duration, Waker>>> = Arc::new(Mutex::new(BTreeMap::new()));
}

pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    SPAWN_QUEUE.push(Task::new(future));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

/// Function will wake up any async futures that have slept for enough.
pub(crate) fn wake_tasks() {
    let mut lock = TIMER_QUEUE.lock();
    let current_milis = get_milis() as u128;

    loop {
        if let Some((k, v)) = lock.pop_first() {
            if k.as_millis() > current_milis {
                super::arch::pit::notify_in(k.as_millis());
                lock.insert(k, v);
                return;
            }

            v.wake();
            super::arch::pit::reset_notify();
        } else {
            return;
        }
    }
}

/// Push a new waker into our timer queue.
fn push_timer(when: Duration, waker: Waker) {
    without_interrupts(move || {
        {
            let mut lock = TIMER_QUEUE.lock();
            lock.insert(when, waker);
        }
        wake_tasks();
    });
}

pub struct Sleep {
    yield_at: Duration,
}

impl Sleep {
    pub fn new(period: Duration) -> Self {
        Self {
            yield_at: Duration::from_millis(get_milis()) + period,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let current_milis = get_milis();

        if current_milis >= self.yield_at.as_millis() as u64 {
            return Poll::Ready(());
        }

        push_timer(self.yield_at.clone(), cx.waker().clone());
        Poll::Pending
    }
}

pub struct Interval {
    period: Duration,
    timer: Option<Sleep>,
}

impl Interval {
    pub fn new(period: Duration) -> Self {
        Self {
            timer: Some(Sleep::new(period.clone())),
            period,
        }
    }

    pub async fn tick(&mut self) {
        self.timer.take().unwrap().await;
        self.timer = Some(Sleep::new(self.period.clone()));
    }
}

pub mod executor;

use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use crossbeam_queue::SegQueue;

lazy_static::lazy_static! {
    pub static ref SPAWN_QUEUE: SegQueue<Task> = SegQueue::new();
}

pub fn spawn(future: impl Future<Output = ()> + Send + Sync + 'static) {
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
    future: Pin<Box<dyn Future<Output = ()> + Send + Sync>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + Send + Sync + 'static) -> Self {
        Self {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

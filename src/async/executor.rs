use super::{Task, TaskId};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::task::Wake;
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;
use x86_64::instructions::interrupts::{self, enable_and_hlt};

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(0xff)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;

        if self.tasks.insert(task.id, task).is_some() {
            panic!("async: task with same ID already in tasks");
        }

        self.task_queue
            .push(task_id)
            .expect("async: task_queue full");
    }

    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(tid) = task_queue.pop() {
            let task = match tasks.get_mut(&tid) {
                Some(task) => task,
                None => continue,
            };

            let waker = waker_cache
                .entry(tid)
                .or_insert_with(|| TaskWaker::new(tid, task_queue.clone()));

            let mut cx = Context::from_waker(waker);
            match task.poll(&mut cx) {
                Poll::Ready(()) => {
                    tasks.remove(&tid);
                    waker_cache.remove(&tid);
                }
                Poll::Pending => {}
            }
        }
    }

    fn merge_spawn_queue(&mut self) {
        while let Some(task) = super::SPAWN_QUEUE.pop() {
            self.spawn(task);
        }
    }

    pub fn run(&mut self) {
        loop {
            self.merge_spawn_queue();
            self.run_ready_tasks();

            interrupts::disable();
            if self.task_queue.is_empty() {
                enable_and_hlt();
            } else {
                interrupts::enable();
            }
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(Self {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue
            .push(self.task_id)
            .expect("async: task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

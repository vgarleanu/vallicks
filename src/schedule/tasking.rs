use crate::gdt::GDT;
use crate::println;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::sync::atomic::AtomicBool;
use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref SCHEDULER: Arc<Mutex<TaskManager>> = Arc::new(Mutex::new(TaskManager::new(256)));
}

pub struct TaskManager {
    max_tasks: u32,
    current_task: usize,
}

impl TaskManager {
    pub fn new(max_tasks: u32) -> Self {
        Self {
            max_tasks,
            current_task: 0,
        }
    }

    pub fn add_task(&mut self, task: fn()) {
        if self.tasks.len() < self.max_tasks as usize {
            let stack = OwnedStack::new(4096).unwrap();
            self.tasks.push(Generator::new(stack, move |_, _| {
                let _ = task();
            }));
        }
    }

    pub fn schedule(&mut self) {
        if self.tasks.len() < 1 {
            return;
        }

        if self.tasks.len() == self.current_task {
            self.current_task = 0;
        }

        self.current_task += 1;
        self.tasks[self.current_task - 1].resume()
    }
}

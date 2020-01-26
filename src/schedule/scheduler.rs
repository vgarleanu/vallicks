use crate::schedule::thread::{Thread, ThreadId};
use alloc::collections::{BTreeMap, VecDeque};
use core::mem;
use x86_64::VirtAddr;
/// Simple round-robin task scheduler
pub struct Scheduler {
    threads: BTreeMap<ThreadId, Thread>,
    current_thread_id: ThreadId,
    paused_threads: VecDeque<ThreadId>,
}

impl Scheduler {
    pub fn new() -> Self {
        let root_thread = Thread::create_root_thread();
        let root_id = root_thread.id();
        let mut threads = BTreeMap::new();

        threads
            .insert(root_id, root_thread)
            .expect_none("map is not empty after creation");

        Scheduler {
            threads,
            current_thread_id: root_id,
            paused_threads: VecDeque::new(),
        }
    }

    fn next_thread(&mut self) -> Option<ThreadId> {
        self.paused_threads.pop_front()
    }

    pub fn schedule(&mut self) -> Option<(ThreadId, VirtAddr)> {
        if let Some(next_id) = self.next_thread() {
            let next_thread = self
                .threads
                .get_mut(&next_id)
                .expect("next thread does not exist");
            let next_stack_pointer = next_thread
                .stack_pointer()
                .take()
                .expect("paused thread has no stack pointer");
            Some((next_id, next_stack_pointer))
        } else {
            None
        }
    }

    pub(super) fn add_paused_thread(
        &mut self,
        paused_stack_pointer: VirtAddr,
        next_thread_id: ThreadId,
    ) -> Result<(), core::option::NoneError> {
        let paused_thread_id = mem::replace(&mut self.current_thread_id, next_thread_id);
        let paused_thread = self.threads.get_mut(&paused_thread_id)?;

        paused_thread
            .stack_pointer()
            .replace(paused_stack_pointer)
            .expect_none("running thread should have stack pointer set to None");
        self.paused_threads.push_back(paused_thread_id);
        Ok(())
    }

    pub fn add_new_thread(&mut self, thread: Thread) {
        let thread_id = thread.id();
        self.threads
            .insert(thread_id, thread)
            .expect_none("thread already exists");
        self.paused_threads.push_back(thread_id);
    }

    pub(super) fn current_thread_id(&self) -> ThreadId {
        self.current_thread_id
    }

    pub(super) fn remove_thread(&mut self, id: ThreadId) {
        let _thread = self.threads.remove(&id);
        self.paused_threads.retain(|&x| x != id);
    }
}

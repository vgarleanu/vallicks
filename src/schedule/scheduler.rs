use crate::{
    prelude::*,
    schedule::thread::{Thread, ThreadId},
};
use alloc::collections::{BTreeMap, VecDeque};
use core::mem;
use x86_64::VirtAddr;

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
            .expect_none("scheduler: map is not empty after creation");

        Scheduler {
            threads,
            current_thread_id: root_id,
            paused_threads: VecDeque::new(),
        }
    }

    fn next_thread(&mut self) -> Option<(ThreadId, &mut Thread)> {
        if let Some(tid) = self.paused_threads.pop_front() {
            if let Some(thread) = self.threads.get_mut(&tid) {
                return Some((tid, thread));
            }

            println!("scheduler: attempted to switch to a thread that doesnt exist");
        }
        None
    }

    pub fn schedule(&mut self) -> Option<(ThreadId, VirtAddr)> {
        if let Some((tid, thread)) = self.next_thread() {
            if !thread.is_ready() {
                self.paused_threads.push_back(tid);
                return None;
            }

            if let Some(sp) = thread.stack_pointer().take() {
                return Some((tid, sp));
            }

            println!("scheduler: thread has no stack pointer, gonna clean");
            self.remove_thread(tid);
        }

        None
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
            .expect_none("scheduler: running thread should have stack pointer set to None");
        self.paused_threads.push_back(paused_thread_id);
        Ok(())
    }

    pub fn add_new_thread(&mut self, thread: Thread) {
        let thread_id = thread.id();
        self.threads
            .insert(thread_id, thread)
            .expect_none("scheduler: attempted to add a thread with a conflicting id");
        self.paused_threads.push_back(thread_id);
    }

    pub(super) fn current_thread_id(&self) -> ThreadId {
        self.current_thread_id
    }

    pub(super) fn remove_thread(&mut self, id: ThreadId) {
        if self.threads.remove(&id).is_none() {
            println!("scheduler: warn attempted to remove thread that doesnt exist in the pool");
        }

        self.paused_threads.retain(|&x| x != id);
    }

    pub(super) fn park_current(&mut self, milis: u64) {
        if let Some(thread) = self.threads.get_mut(&self.current_thread_id) {
            thread.park(milis);
            return;
        }

        println!(
            "scheduler: Attempted to park a thread that doesnt exist with TID: {}",
            self.current_thread_id.as_u64()
        );

        self.remove_thread(self.current_thread_id);
    }

    pub(super) fn mark_dirty(&mut self, panic_info: String) {
        let id = self.current_thread_id();
        println!("scheduler::warn marking thread {} as dirty", id.as_u64());

        match self.threads.remove(&id) {
            Some(mut x) => x.set_panicking(panic_info),
            None => println!("scheduler: a thread that doesnt exist panic'd"),
        }
    }
}

use crate::{
    prelude::*,
    schedule::thread::{Thread, ThreadId},
};
use alloc::collections::{BTreeMap, VecDeque};
use core::mem;
use x86_64::VirtAddr;

/// Struct represents our scheduler and holds all the data required for switching between tasks.
/// The scheduler operates in a round robin fashion.
pub struct Scheduler {
    /// This is our list of threads we want to execute.
    threads: BTreeMap<ThreadId, Thread>,
    /// This is the id of the thread that is executing currently.
    current_thread_id: ThreadId,
    /// This is a deque of all the paused threads. When we switch from one thread to another, the
    /// previous thread gets put into this VecDeque to be later popped off and executed.
    paused_threads: VecDeque<ThreadId>,
}

impl Scheduler {
    /// Method returns a new instance of a scheduler. Technically speaking this method only ever
    /// gets called once during kernel init, or it never gets called if the scheduler is disabled.
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

    /// Method tries to pop a paused thread from our VecDeque and return it as a tuple of its
    /// Unique ID and the thread itself as a mutable reference.
    fn next_thread(&mut self) -> Option<(ThreadId, &mut Thread)> {
        if let Some(tid) = self.paused_threads.pop_front() {
            if let Some(thread) = self.threads.get_mut(&tid) {
                return Some((tid, thread));
            }

            println!("scheduler: attempted to switch to a thread that doesnt exist");
        }
        None
    }

    /// This is the method that does all the magic. The method grabs a paused thread, if there is
    /// none then it just returns None. Then it checks if the thread is ready to be executed. If
    /// the thread is ready to be executed it returns  a tuple of the ID of the thread and its
    /// stack pointer. This is later used to do a context switch.
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

    /// This method pushes the current thread into the paused threads deque.
    ///
    /// # Arguments
    /// * `paused_stack_pointer` - This is the new paused stack pointer of our thread.
    /// * `next_thread_id` - This is the id of the next thread that will be executed.
    pub fn add_paused_thread(
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

    /// Method adds a new thread to be executed later.
    ///
    /// # Arguments
    /// * `thread` - The new thread to be executed in the future
    pub fn add_new_thread(&mut self, thread: Thread) {
        let thread_id = thread.id();
        self.threads
            .insert(thread_id, thread)
            .expect_none("scheduler: attempted to add a thread with a conflicting id");
        self.paused_threads.push_back(thread_id);
    }

    /// Method returns the ID of the thread executing in the very current moment.
    pub fn current_thread_id(&self) -> ThreadId {
        self.current_thread_id
    }

    /// Method removes the thread with the ID supplies from the scheduler, essentially cancelling
    /// its execution.
    ///
    /// # Safety
    /// Be careful when using this method because if the threads handle isnt informed that the
    /// thread stopped executing, will cause a infinite loop when joining.
    ///
    /// # Arguments
    /// * `id` - The id of the thread we wish to kill.
    pub fn remove_thread(&mut self, id: ThreadId) {
        if self.threads.remove(&id).is_none() {
            println!("scheduler: warn attempted to remove thread that doesnt exist in the pool");
        }

        self.paused_threads.retain(|&x| x != id);
    }

    /// Method parks the current thread for `milis` number of miliseconds.
    ///
    /// # Arguments
    /// * `milis` - number of miliseconds to sleep for
    pub fn park_current(&mut self, milis: u64) {
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

    /// Method marks the current thread as dirty. This is only necessary when the thread has
    /// unexpectedly panicked.
    /// When the thread panics, the panic handler will call this method and pass it the panic_info
    /// as a string. The scheduler then removes the thread from the task list, the thread is then
    /// set as panicking. This has two side effects, one is that the panic info is dispatched to
    /// our JoinHandle, then the JoinHandle is informed that the thread has finished execution.
    /// When the JoinHandle is joined, it is supposed to return a `Err()` with our panic info.
    ///
    /// # Arguments
    /// * `panic_info` - This is the message passed to our panic handler giving some info
    pub fn mark_dirty(&mut self, panic_info: String) {
        let id = self.current_thread_id();
        println!("scheduler::warn marking thread {} as dirty", id.as_u64());

        backtrack();

        match self.threads.remove(&id) {
            Some(mut x) => x.set_panicking(panic_info),
            None => println!("scheduler: a thread that doesnt exist panic'd"),
        }
    }
}

use crate::arch::memory::translate_addr;
/// Function walks the base pointer, yielding a backtrace, however it may be incomplete.
pub fn backtrack() {
    println!("Backtrace:");
    let mut base_pointer: *const usize;

    // Get the address of pushed base pointer
    unsafe { asm!("mov rax, rbp" : "={rax}"(base_pointer) ::: "intel") }

    // Before entering boot_entry we set the base pointer to null (0)
    // This way, we can determine when to stop walking the stack
    // See the start64_2 function in boot_entry.asm
    while !base_pointer.is_null() || base_pointer as usize > 0x10000 {
        if unsafe { translate_addr(VirtAddr::from_ptr(base_pointer)) }.is_none() {
            break;
        }

        // The return address is above the pushed base pointer
        let return_address = unsafe { *(base_pointer.offset(1)) } as usize;

        // If we haven't loaded the symbol table yet just
        // print the raw return address
        println!("    > {:#x}", return_address);

        // The pushed base pointer is the address to the previous stack frame
        base_pointer = unsafe { (*base_pointer) as *const usize };
    }
}

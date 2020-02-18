pub mod scheduler;
pub mod stack;
pub mod switch;
pub use crate::naked_std::thread;

use crate::{globals::SCHEDULER, prelude::*, schedule::scheduler::Scheduler};
use switch::context_switch_to;
use thread::{Thread, ThreadId};

/// Method creates a new scheduler instance and sets it to the global named `SCHEDULER`. This
/// method should only be ever called once.
pub fn init_scheduler() {
    let mut lock = SCHEDULER.lock();
    *lock = Some(Scheduler::new());

    println!("scheduler: Scheduler setup done...");
}

/// Methood tries to lock the global scheduler mutex, if successful it grabs the next task from the
/// scheduler then context switches to it.
///
/// This method is only called internally by our timer interrupt handler. If you want to yield the
/// current thread, you should use the `naked_std::thread::yield_now` method.
pub fn schedule() {
    let next = SCHEDULER
        .try_lock()
        .and_then(|mut scheduler| scheduler.as_mut().and_then(|s| s.schedule()));
    if let Some((next_id, next_stack_pointer)) = next {
        // We dont actually care if theres no paused thread
        unsafe {
            let _ = context_switch_to(next_id, next_stack_pointer);
        };
    }
}

/// Method returns the id of the currently running thread. This method is only ever called by the
/// methods inside the `naked_std::thread`.
pub fn current_thread_id() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("schedule::current: SCHEDULER is none, BUG");
    }

    slock.as_mut().unwrap().current_thread_id()
}

/// Method safely returns the ThreadId. If this method is called before the scheduler is
/// initialized it returns a default ThreadId which is 0. It is useful in our panic handler, if our
/// unikernel panics during the init phase.
pub fn safe_current_thread_id() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        return ThreadId::default();
    }

    slock.as_mut().unwrap().current_thread_id()
}

/// This method removes the current thread from the scheduler, making the thread essentially stop
/// executing. This is only ever called when the thread has either finished running and has freed
/// all resources, or when the thread has panic'd and needs to be quickly removed from the
/// execution stack to avoid memory leaks or resource clogs.
pub fn remove_self() {
    let mut slock = SCHEDULER.lock();
    let scheduler = slock.as_mut().unwrap();

    let current = scheduler.current_thread_id();
    scheduler.remove_thread(current);
}

/// This method is used internally by `naked_std::thread` to add a new thread context to the
/// scheduler to be executed. Once this method is called, the code closure within the thread will
/// be executed.
///
/// While this method isn't in itself unsafe, it has been marked as unsafe because the caller needs
/// to ensure that the stack has been set up properly to avoid kernel memory corruption.
///
/// # Arguments
/// * `t` - Thread
pub unsafe fn add_new_thread(t: Thread) {
    let mut slock = SCHEDULER.lock();
    slock.as_mut().unwrap().add_new_thread(t);
}

/// Method locks the scheduler and tries to grab the next task to execute. If there is a next task
/// it then does a context switch.
pub fn yield_now() {
    let next = {
        let mut slock = SCHEDULER.lock();
        slock.as_mut().unwrap().schedule()
    };
    if let Some((next_id, next_stack_pointer)) = next {
        unsafe {
            let _ = context_switch_to(next_id, next_stack_pointer);
        };
    }
}

/// This method parks the current thread for milis number of miliseconds. It tries to continously
/// grab a next thread to context switch into. Once it has acquired a task, it does a context
/// switch, once complete it breaks and returns
///
/// # Arguments
/// * `milis` - Miliseconds to sleep for
pub fn park_current(milis: u64) {
    loop {
        let next = {
            let mut slock = SCHEDULER.lock();
            slock.as_mut().unwrap().park_current(milis);
            slock.as_mut().unwrap().schedule()
        };

        if let Some((next_id, next_stack_pointer)) = next {
            unsafe {
                let _ = context_switch_to(next_id, next_stack_pointer);
            };
            break;
        }
        unsafe {
            asm!("hlt" :::: "volatile");
        }
    }
}

/// Method used internally by the panic handler to mark the current thread as dirty. This is
/// necessary when a thread panics and its resources need to be freed
///
/// # Arguments
/// * `panic_info` - String containing the panic message from the thread
pub fn mark_dirty(panic_info: String) {
    let mut slock = SCHEDULER.lock();
    slock.as_mut().unwrap().mark_dirty(panic_info);
}

pub mod scheduler;
pub mod stack;
pub mod switch;
pub use crate::naked_std::thread;

use crate::{
    globals::{FRAME_ALLOCATOR, MAPPER, SCHEDULER},
    prelude::*,
    schedule::scheduler::Scheduler,
    sync::Arc,
};
use switch::context_switch_to;
use thread::{JoinHandle, Thread, ThreadId};

pub fn init_scheduler() {
    let mut lock = SCHEDULER.lock();
    *lock = Some(Scheduler::new());

    println!("scheduler: Scheduler setup done...");
}

pub(crate) fn schedule() {
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

pub fn current_thread_id() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("schedule::current: SCHEDULER is none, BUG");
    }

    slock.as_mut().unwrap().current_thread_id()
}

pub fn safe_current_thread_id() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        return ThreadId::default();
    }

    slock.as_mut().unwrap().current_thread_id()
}

pub fn remove_self() {
    let mut slock = SCHEDULER.lock();
    let mut scheduler = slock.as_mut().unwrap();

    let current = scheduler.current_thread_id();
    scheduler.remove_thread(current);
}

pub unsafe fn add_new_thread(t: Thread) {
    let mut slock = SCHEDULER.lock();
    slock.as_mut().unwrap().add_new_thread(t);
}

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

pub fn is_aborted() -> bool {
    false
}

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

pub fn mark_dirty(panic_info: String) {
    let mut slock = SCHEDULER.lock();
    slock.as_mut().unwrap().mark_dirty(panic_info);
}

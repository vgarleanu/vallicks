pub mod scheduler;
pub mod stack;
pub mod switch;
pub mod thread;

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

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("schedule::spawn: SCHEDULER is none, BUG");
    }

    let mut mlock = MAPPER.lock();
    let mut alock = FRAME_ALLOCATOR.lock();

    let mapper = mlock.as_mut();
    let alloc = alock.as_mut();

    let mut handle: JoinHandle<T> = JoinHandle::new();
    let inner = handle.get_inner();
    let mut switch = handle.get_switch();

    let thread = Thread::new(
        move || {
            let thread_id = {
                let lock = SCHEDULER.lock();
                lock.as_ref().unwrap().current_thread_id()
            };

            let ret = f();
            unsafe {
                Arc::get_mut_unchecked(&mut switch).switch();
                *inner.0.get() = Some(ret);
            }

            {
                let mut lock = SCHEDULER.lock();
                lock.as_mut().unwrap().remove_thread(thread_id);
            }

            loop {
                x86_64::instructions::hlt()
            }
        },
        2,
        mapper.unwrap(),
        alloc.unwrap(),
    )
    .expect("scheduler: failed to spawn a thread");

    slock.as_mut().unwrap().add_new_thread(thread);
    handle
}

pub fn current() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("schedule::current: SCHEDULER is none, BUG");
    }

    slock.as_mut().unwrap().current_thread_id()
}

pub fn sleep(milis: u64) {
    loop {
        let next = {
            let mut slock = SCHEDULER.lock();
            slock.as_mut().unwrap().park_current(milis);
            slock.as_mut().unwrap().schedule()
        };

        if let Some((next_id, next_stack_pointer)) = next {
            // We dont actually care if theres no paused thread
            unsafe {
                let _ = context_switch_to(next_id, next_stack_pointer);
            };
            break;
        }

        // We halt for a cycle to not eat up the cpu
        unsafe {
            asm!("hlt" :::: "volatile");
        }
    }
}

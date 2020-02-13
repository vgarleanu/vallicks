use crate::prelude::*;
use crate::arch::memory::BootInfoFrameAllocator;
use crate::schedule::scheduler::Scheduler;
use spin::Mutex;
use x86_64::structures::paging::mapper::OffsetPageTable;

pub mod scheduler;
pub mod stack;
pub mod switch;
pub mod thread;

use switch::context_switch_to;
use thread::{Thread, ThreadId};

pub(super) static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

// TODO: Move these into lib.rs or arch/mod.rs
pub(crate) static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
pub(crate) static ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

pub fn init_scheduler(
    mapper: OffsetPageTable<'static>,
    frame_allocator: BootInfoFrameAllocator,
) {
    let mut lock = SCHEDULER.lock();
    *lock = Some(Scheduler::new());

    let mut lock = MAPPER.lock();
    *lock = Some(mapper);

    let mut lock = ALLOCATOR.lock();
    *lock = Some(frame_allocator);
    println!("[SCHED] Scheduler setup done...");
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

pub fn spawn<F, T>(f: F)
where
    F: FnOnce() -> T,
    F: Send + Sync + 'static,
    T: Send + 'static,
{
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("");
    }

    let mut mlock = MAPPER.lock();
    let mut alock = ALLOCATOR.lock();

    let mapper = mlock.as_mut();
    let alloc = alock.as_mut();

    let thread = Thread::new(
        || {
            let thread_id = {
                let lock = SCHEDULER.lock();
                lock.as_ref().unwrap().current_thread_id()
            };

            f();

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
    .unwrap();
    slock.as_mut().unwrap().add_new_thread(thread);
}

pub fn current() -> ThreadId {
    let mut slock = SCHEDULER.lock();
    if slock.is_none() {
        panic!("");
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
    }
}

use crate::memory::BootInfoFrameAllocator;
use crate::schedule::scheduler::Scheduler;
use crate::{print, println};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::paging::mapper::OffsetPageTable;
use x86_64::VirtAddr;

pub mod scheduler;
pub mod stack;
pub mod switch;
pub mod thread;

use switch::context_switch_to;
use thread::Thread;

pub(super) static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);
pub(super) static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
pub(super) static ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

pub fn init_scheduler(mapper: OffsetPageTable<'static>, frame_allocator: BootInfoFrameAllocator) {
    let mut lock = SCHEDULER.lock();
    *lock = Some(Scheduler::new());

    let mut lock = MAPPER.lock();
    *lock = Some(mapper);

    let mut lock = ALLOCATOR.lock();
    *lock = Some(frame_allocator);
    println!("Scheduler done...");
}

pub fn schedule() {
    let next = SCHEDULER
        .try_lock()
        .and_then(|mut scheduler| scheduler.as_mut().and_then(|s| s.schedule()));
    if let Some((next_id, next_stack_pointer)) = next {
        unsafe { context_switch_to(next_id, next_stack_pointer) };
        return;
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

    let thread = Thread::create_from_closure(
        || {
            f();
            // TODO: Inform scheduler that the task is done
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

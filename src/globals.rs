use crate::arch::memory::BootInfoFrameAllocator;
use crate::prelude::sync::Mutex;
use crate::prelude::*;
use crate::schedule::scheduler::Scheduler;
use buddy_system_allocator::{Heap, LockedHeapWithRescue};
use x86_64::structures::paging::mapper::OffsetPageTable;

#[global_allocator]
pub static ALLOCATOR: LockedHeapWithRescue = LockedHeapWithRescue::new(|heap: &mut Heap| {
    let (start, size) = match crate::arch::allocator::extend_heap() {
        Ok(x) => x,
        Err(e) => panic!("{}", e),
    };

    println!(
        "allocator: assigning extra heap @ {:#x}...{:#x}",
        start,
        start + size
    );
    unsafe {
        heap.add_to_heap(start, start + size);
    }
});
pub static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);
pub static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
pub static FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

pub(crate) fn extend_alloc_heap(start: usize, size: usize) {
    unsafe {
        ALLOCATOR.lock().add_to_heap(start, start + size);
    }
}

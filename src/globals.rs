use crate::arch::memory::BootInfoFrameAllocator;
use buddy_system_allocator::{Heap, LockedHeapWithRescue};
use spin::Mutex;
use x86_64::structures::paging::mapper::OffsetPageTable;

/// This is our global default allocator, at the moment we only feature gate the
/// buddy_system_allocator allocator but in the future we will offer more allocator types.
/// We create a `LockedHeapWithRescue` allocator which allows us to catch out-of-memory errors and
/// extend the heap accordingly. The inner closure tries to extend the heap and if successful
/// informs the inner heap.
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

/// This is our global page mapper
pub(crate) static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
/// This is our global frame allocator
pub(crate) static FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

/// Method allows us to lock the heap and map additional memory to it.
///
/// # Arguments
/// * `start` - The start address of our new heap allocation
/// * `size` - The size of our new heap allocation
pub(crate) fn extend_alloc_heap(start: usize, size: usize) {
    unsafe {
        ALLOCATOR.lock().add_to_heap(start, start + size);
    }
}

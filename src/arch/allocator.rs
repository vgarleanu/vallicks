//! This is our allocator module, that is used internally for two things.
//! * Create the initial heap that will allow us to store things on the heap with ease.
//! * Extend the heap when we exceed the assigned range
use crate::globals;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

static mut HEAP_LAST: usize = 0x4444_4444_0000;
const HEAP_SIZE: usize = 1024 * 1024;

fn assign_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(usize, usize), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(unsafe { HEAP_LAST } as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        HEAP_LAST += HEAP_SIZE;
    }

    Ok((unsafe { HEAP_LAST } - HEAP_SIZE, HEAP_SIZE))
}

/// Function extends the heap and returns a tuple with the start address of the new heap and the
/// size of the new heap. If an error occured it returns the error message that later gets
/// forwarded to a panic.
pub fn extend_heap() -> Result<(usize, usize), &'static str> {
    let mut mlock = globals::MAPPER.lock();
    let mapper = mlock
        .as_mut()
        .map_or_else(|| Err("allocator: tried to lock a empty mapper"), |x| Ok(x))?;

    let mut flock = globals::FRAME_ALLOCATOR.lock();
    let frame_allocator = flock.as_mut().map_or_else(
        || Err("allocator: tried to lock a empty frame allocator"),
        |x| Ok(x),
    )?;

    assign_heap(mapper, frame_allocator).map_err(|_| "allocator: failed to extend the heap")
}

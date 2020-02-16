use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

static mut HEAP_LAST: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024;

fn assign_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(usize, usize), MapToError> {
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
        mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    }

    unsafe {
        HEAP_LAST += HEAP_SIZE;
    }

    Ok((unsafe { HEAP_LAST } - HEAP_SIZE, HEAP_SIZE))
}

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError> {
    let (start, size) = assign_heap(mapper, frame_allocator)?;

    unsafe {
        crate::ALLOCATOR.lock().init(start, size);
    }

    Ok(())
}

pub fn extend_heap() -> (usize, usize) {
    let mut mlock = crate::schedule::MAPPER.lock();
    let mapper = mlock.as_mut().expect("Mlock was empty lol");

    let mut flock = crate::schedule::ALLOCATOR.lock();
    let frame_allocator = flock.as_mut().expect("Flock is empty");

    match assign_heap(mapper, frame_allocator) {
        Ok((start, size)) => return (start, size),
        Err(e) => panic!("allocator: failed to extend the heap with err {:#?}", e),
    }
}

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags as Flags,
        PhysFrame, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};

pub struct BootInfoFrameAllocator {
    mem_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(mem_map: &'static MemoryMap) -> Self {
        Self { mem_map, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = UnusedPhysFrame> {
        let regions = self.mem_map.iter();
        let usable_regions = regions.filter(|x| x.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|x| x.range.start_addr()..x.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|x| x.step_by(0x1000));
        let frames = frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));
        frames.map(|f| unsafe { UnusedPhysFrame::new(f) })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));

    let unused_frame = unsafe { UnusedPhysFrame::new(frame) };
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = mapper.map_to(page, unused_frame, flags, frame_allocator);
    map_to_result.expect("map_to failed").flush();
}

/// Initialize a new OffsetPageTable
pub unsafe fn init(physical_mem_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_l4_table(physical_mem_offset);
    OffsetPageTable::new(level_4_table, physical_mem_offset)
}

/// Returns mut reference to a active l4 table
///
/// This function is unsafe because the calee must ensure the
/// physical memory is mapped to the virtual memory at the passed
/// `physical_mem_offset`. This function must also be only called
/// once to avoid aliasing `&mut` references.
unsafe fn active_l4_table(physical_mem_offset: VirtAddr) -> &'static mut PageTable {
    let (l4_table_frame, _) = Cr3::read();

    let phys = l4_table_frame.start_address();
    let virt = physical_mem_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackBounds {
    start: VirtAddr,
    end: VirtAddr,
}

impl StackBounds {
    pub fn start(&self) -> VirtAddr {
        self.start
    }

    pub fn end(&self) -> VirtAddr {
        self.end
    }
}

pub fn alloc_stack(
    size_in_pages: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<StackBounds, mapper::MapToError> {
    static STACK_ALLOC_NEXT: AtomicU64 = AtomicU64::new(0x_5555_5555_0000);

    let guard_page_start = STACK_ALLOC_NEXT.fetch_add(
        (size_in_pages + 1) * Page::<Size4KiB>::SIZE,
        Ordering::SeqCst,
    );
    let guard_page = Page::from_start_address(VirtAddr::new(guard_page_start))
        .expect("`STACK_ALLOC_NEXT` not page aligned");

    let stack_start = guard_page + 1;
    let stack_end = stack_start + size_in_pages;
    let flags = Flags::PRESENT | Flags::WRITABLE;
    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(mapper::MapToError::FrameAllocationFailed)?;
        mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    }
    Ok(StackBounds {
        start: stack_start.start_address(),
        end: stack_end.start_address(),
    })
}

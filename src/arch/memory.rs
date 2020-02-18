//! This module contains all the kernel memory related functions.
//! This is the home to the Paging init functions and the frame allocator.
#![allow(missing_docs)]
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper, page_table::FrameError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags as Flags, PhysFrame, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};

static mut MEM_OFFSET: Option<VirtAddr> = None;

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

pub unsafe fn init(physical_mem_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_l4_table(physical_mem_offset);
    MEM_OFFSET = Some(physical_mem_offset);
    OffsetPageTable::new(level_4_table, physical_mem_offset)
}

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
    static STACK_ALLOC_NEXT: AtomicU64 = AtomicU64::new(0x5555_5555_0000);

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

pub unsafe fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();
    let mem_offset = MEM_OFFSET.expect("MEM_OFFSET not init");

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi-level page table
    for &index in &table_indexes {
        // convert the frame into a page table reference
        let virt = mem_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = &*table_ptr;

        // read the page table entry and update `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // calculate the physical address by adding the page offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

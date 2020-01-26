use crate::memory::{alloc_stack, StackBounds};
use crate::schedule::stack::Stack;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{
    structures::paging::{mapper, FrameAllocator, Mapper, Size4KiB},
    VirtAddr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(u64);

impl ThreadId {
    fn new() -> Self {
        static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
        ThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::SeqCst))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<ThreadId> for u64 {
    fn from(f: ThreadId) -> u64 {
        f.as_u64()
    }
}

#[derive(Debug)]
pub struct Thread {
    id: ThreadId,
    pub parked: Option<(u64, u64)>,
    stack_pointer: Option<VirtAddr>,
    stack_bounds: Option<StackBounds>,
}

impl Thread {
    pub fn new<F>(
        closure: F,
        stack_size: u64,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<Self, mapper::MapToError>
    where
        F: FnOnce() -> ! + 'static + Send + Sync,
    {
        let stack_bounds = alloc_stack(stack_size, mapper, frame_allocator)?;
        let mut stack = unsafe { Stack::new(stack_bounds.end()) };

        stack.set_up_for_closure(Box::new(closure));

        Ok(Self {
            id: ThreadId::new(),
            parked: None,
            stack_pointer: Some(stack.get_stack_pointer()),
            stack_bounds: Some(stack_bounds),
        })
    }

    pub fn create_root_thread() -> Self {
        Self {
            id: ThreadId(0),
            parked: None,
            stack_pointer: None,
            stack_bounds: None,
        }
    }

    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn stack_pointer(&mut self) -> &mut Option<VirtAddr> {
        &mut self.stack_pointer
    }
}

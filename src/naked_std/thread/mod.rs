use crate::{
    arch::{
        memory::{alloc_stack, StackBounds},
        pit::get_milis,
    },
    prelude::*,
    schedule as scheduler,
    schedule::stack::Stack,
    sync::Arc,
};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use x86_64::{
    structures::paging::{mapper, FrameAllocator, Mapper, Size4KiB},
    VirtAddr,
};

pub struct Builder {
    name: Option<String>,
    stack_size: Option<u64>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            name: None,
            stack_size: None,
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn stack_size(mut self, stack_size: u64) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    pub fn spawn<F, T>(self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let mut handle: JoinHandle<T> = JoinHandle::new();
        let mut switch = handle.get_switch();
        let mut panic_state = handle.get_panic();
        let inner = handle.get_inner();

        let thread = Thread::new(
            move || {
                unsafe {
                    *inner.0.get() = Some(f());
                    Arc::get_mut_unchecked(&mut switch).switch();
                }

                scheduler::remove_self();

                loop {
                    x86_64::instructions::hlt();
                }
            },
            self.stack_size.unwrap_or(2),
            panic_state,
            handle.get_switch(),
        );

        unsafe {
            scheduler::add_new_thread(thread.unwrap());
        }

        handle
    }
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    Builder::new().spawn(f)
}

pub fn current() -> ThreadId {
    scheduler::current_thread_id()
}

pub fn yield_now() {
    scheduler::yield_now()
}

pub fn panicking() -> bool {
    scheduler::is_aborted()
}

pub fn sleep(ms: u64) {
    scheduler::park_current(ms);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(u64);

impl ThreadId {
    fn new() -> Self {
        static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
        ThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::SeqCst))
    }

    pub(crate) fn default() -> Self {
        ThreadId(0)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct Packet<T>(Arc<UnsafeCell<Option<T>>>);

impl<T> Packet<T> {
    fn new() -> Self {
        Self(Arc::new(UnsafeCell::new(None)))
    }
}

impl<T> Clone for Packet<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

unsafe impl<T: Send> Send for Packet<T> {}
unsafe impl<T: Sync> Sync for Packet<T> {}

pub struct Switch(AtomicBool);

impl Switch {
    pub fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    pub fn switch(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }

    pub fn is_alive(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct JoinHandle<T> {
    alive: Arc<Switch>,
    inner: Packet<T>,
    panic_state: Packet<String>,
}

impl<T> JoinHandle<T> {
    pub fn new() -> Self {
        Self {
            alive: Arc::new(Switch::new()),
            inner: Packet::new(),
            panic_state: Packet::new(),
        }
    }

    pub fn join(mut self) -> Result<T, String> {
        loop {
            if !self.alive.is_alive() {
                match unsafe { (*self.panic_state.0.get()).take() } {
                    Some(x) => return Err(x),
                    None => return unsafe { Ok((*self.inner.0.get()).take().unwrap()) },
                }
            }
        }
    }

    pub(super) fn get_inner(&self) -> Packet<T> {
        self.inner.clone()
    }

    pub(super) fn get_panic(&self) -> Packet<String> {
        self.panic_state.clone()
    }

    pub(super) fn get_switch(&self) -> Arc<Switch> {
        self.alive.clone()
    }
}

pub struct Thread {
    id: ThreadId,
    parked: Option<(u64, u64)>,
    stack_pointer: Option<VirtAddr>,
    stack_bounds: Option<StackBounds>,
    panic_state: Packet<String>,
    switch: Arc<Switch>,
}

impl Thread {
    pub fn new<F>(
        closure: F,
        stack_size: u64,
        panic_state: Packet<String>,
        switch: Arc<Switch>,
    ) -> Result<Self, mapper::MapToError>
    where
        F: FnOnce() -> !,
        F: Send + Sync + 'static,
    {
        let mut mapper = crate::globals::MAPPER.lock();
        let mut frame_allocator = crate::globals::FRAME_ALLOCATOR.lock();

        let stack_bounds = alloc_stack(
            stack_size,
            mapper.as_mut().unwrap(),
            frame_allocator.as_mut().unwrap(),
        )?;
        let mut stack = unsafe { Stack::new(stack_bounds.end()) };
        println!(
            "scheduler: new thread stack @ {:#x}..{:#x}",
            stack_bounds.start().as_u64(),
            stack_bounds.end().as_u64()
        );

        stack.set_up_for_closure(Box::new(closure));

        Ok(Self {
            id: ThreadId::new(),
            parked: None,
            stack_pointer: Some(stack.get_stack_pointer()),
            stack_bounds: Some(stack_bounds),
            panic_state,
            switch,
        })
    }

    pub fn create_root_thread() -> Self {
        Self::new_root_thread()
    }

    pub fn new_root_thread() -> Self {
        Self {
            id: ThreadId(0),
            parked: None,
            stack_pointer: None,
            stack_bounds: None,
            panic_state: Packet::new(), // we dont actually care
            switch: Arc::new(Switch::new()),
        }
    }

    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn stack_pointer(&mut self) -> &mut Option<VirtAddr> {
        &mut self.stack_pointer
    }

    pub fn is_ready(&mut self) -> bool {
        if let Some((parked_at, for_milis)) = self.parked {
            if get_milis() < parked_at + for_milis {
                return false;
            }
            self.parked = None;
        }
        true
    }

    pub fn park(&mut self, milis: u64) {
        self.parked = Some((get_milis(), milis));
    }

    pub(crate) fn set_panicking(&mut self, reason: String) {
        unsafe {
            *self.panic_state.0.get() = Some(reason);
            Arc::get_mut_unchecked(&mut self.switch).switch();
        }
    }
}

impl core::fmt::Debug for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Thread {{ id: {:?}, parked: {:?}, stack_pointer: {:?}, stack_bounds: {:?} }}",
            self.id, self.parked, self.stack_pointer, self.stack_bounds
        )
    }
}

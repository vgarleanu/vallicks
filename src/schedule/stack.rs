use crate::schedule::switch::call_closure_entry;
use alloc::boxed::Box;
use core::mem;
use core::raw::TraitObject;
use x86_64::VirtAddr;

pub struct Stack {
    pointer: VirtAddr,
}

impl Stack {
    pub unsafe fn new(stack_pointer: VirtAddr) -> Self {
        Stack {
            pointer: stack_pointer,
        }
    }

    pub fn get_stack_pointer(self) -> VirtAddr {
        self.pointer
    }

    pub fn set_up_for_closure(&mut self, closure: Box<dyn FnOnce() -> !>) {
        let trait_object: TraitObject = unsafe { mem::transmute(closure) };
        unsafe { self.push(trait_object.data) };
        unsafe { self.push(trait_object.vtable) };

        self.set_up_for_entry_point(call_closure_entry);
    }

    pub fn set_up_for_entry_point(&mut self, entry_point: fn() -> !) {
        unsafe { self.push(entry_point) };
        let rflags: u64 = 0x200;
        unsafe { self.push(rflags) };
    }

    unsafe fn push<T>(&mut self, value: T) {
        self.pointer -= core::mem::size_of::<T>();
        let ptr: *mut T = self.pointer.as_mut_ptr();
        ptr.write(value);
    }
}

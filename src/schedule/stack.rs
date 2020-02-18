use crate::prelude::*;
use crate::schedule::switch::call_closure_entry;
use core::mem;
use core::raw::TraitObject;
use x86_64::VirtAddr;

/// This is the structure holding our stack pointer used by scheduler to start/resume the execution
/// of a thread.
pub struct Stack {
    /// The pointer to the stack as a virtual address.
    pointer: VirtAddr,
}

impl Stack {
    /// Method creates a new stack.
    ///
    /// # Safety
    /// While not directly unsafe, it has been marked as unsafe because the caller must ensure that
    /// the stack pointer actually exist and on access will not cause a page fault.
    pub unsafe fn new(stack_pointer: VirtAddr) -> Self {
        Stack {
            pointer: stack_pointer,
        }
    }

    /// Method consumes self and returns the inner stack pointer
    pub fn get_stack_pointer(self) -> VirtAddr {
        self.pointer
    }

    /// Method sets up the stack pointer to by the stack pointer for the execution of the thread.
    /// To do this it first transmutes the closure to a TraitObject that can be later transmuted
    /// back and invoked. The data and the vtable is the pushed onto the stack.
    /// Lastly we set up the entry point.
    pub fn set_up_for_closure(&mut self, closure: Box<dyn FnOnce() -> !>) {
        let trait_object: TraitObject = unsafe { mem::transmute(closure) };
        unsafe { self.push(trait_object.data) };
        unsafe { self.push(trait_object.vtable) };

        self.set_up_for_entry_point(call_closure_entry);
    }

    /// Method sets the entry point of the thread by pushing the naked function entry_point onto
    /// the stack and setting the rflags to 0x200.
    ///
    /// TODO: What the fuck is rflags 0x200 for??
    pub fn set_up_for_entry_point(&mut self, entry_point: fn() -> !) {
        unsafe { self.push(entry_point) };
        let rflags: u64 = 0x200;
        unsafe { self.push(rflags) };
    }

    /// Method allows us to push something onto the stack, the method decrements the stack pointer
    /// by the size of the data its trying to push, then pushes the new data.
    unsafe fn push<T>(&mut self, value: T) {
        self.pointer -= core::mem::size_of::<T>();
        let ptr: *mut T = self.pointer.as_mut_ptr();
        ptr.write(value);
    }
}

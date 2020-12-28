#[allow(unused_imports)]
pub use crate::{print, println, sprint, sprintln};

#[cfg(test)]
pub use crate::uprint;

pub use alloc::{boxed::Box, format, string::String, vec::Vec};

// TODO: Make this exit thread based instead
#[allow(unused_imports)]
/// This is just our exit prelude used internally by some functions
pub mod exit {
    use crate::exit;
    use crate::ExitCode;
}

pub use crate::hlt_loop as halt;

/// This is out timer module
pub mod timer {
    pub use crate::arch::pit::get_milis;
}

/// This module provides basic primitives for thread sync
pub mod sync {
    pub use alloc::sync::Arc;
    pub use spin::{Mutex, MutexGuard, RwLock};
}

pub use vallicks_macros::{compile_warning, main as entrypoint, unittest};

pub use alloc::vec;

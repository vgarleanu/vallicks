// reimport the core prelude
pub use core::prelude::v1::*;

pub use crate::{print, println, sprint, sprintln};

#[cfg(test)]
pub use crate::uprint;

pub use alloc::borrow::ToOwned;
pub use alloc::boxed::Box;
pub use alloc::format;
pub use alloc::string::String;
pub use alloc::string::ToString;
pub use alloc::vec;
pub use alloc::vec::Vec;

/// This is just our exit prelude used internally by some functions
pub mod exit {
    pub use crate::exit;
    pub use crate::ExitCode;
}

pub use crate::hlt_loop as halt;

/// This is out timer module
pub mod timer {
    pub use crate::arch::pit::get_milis;
}

pub use vallicks_macros::{compile_warning, main as entrypoint, unittest};

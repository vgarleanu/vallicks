#[allow(unused_imports)]
pub use crate::{print, println, sprint, sprintln};

pub use alloc::{boxed::Box, format, string::String, vec::Vec};

// TODO: Make this exit thread based instead
#[allow(unused_imports)]
pub mod exit {
    use crate::exit;
    use crate::ExitCode;
}

pub use crate::hlt_loop as halt;

pub mod timer {
    pub use crate::arch::pit::get_milis;
}

pub mod sync {
    pub use alloc::sync::Arc;
    pub use spin::{Mutex, RwLock};
}

pub use vallicks_macros::{compile_warning, main as entrypoint};

pub use alloc::vec;

#[allow(unused_imports)]
pub use crate::{print, println, sprint, sprintln};

pub use alloc::{boxed::Box, string::String, vec::Vec};

#[allow(unused_imports)]
pub use crate::schedule as thread;

// TODO: Make this exit thread based instead
#[allow(unused_imports)]
pub mod exit {
    use crate::exit;
    use crate::ExitCode;
}

pub use crate::hlt_loop as halt;

pub use crate::arch::interrupts::pop_buffer as input;

pub mod timer {
    pub use crate::arch::pit::get_milis;
}

pub use vallicks_macros::main as entrypoint;

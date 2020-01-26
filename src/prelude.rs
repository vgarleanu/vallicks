#[allow(unused_imports)]
pub use crate::{hlt_loop as halt, print, println, sprint, sprintln};

#[allow(unused_imports)]
pub use crate::schedule as thread;

// TODO: Make this exit thread based instead
#[allow(unused_imports)]
pub mod exit {
    use crate::exit;
    use crate::ExitCode;
}

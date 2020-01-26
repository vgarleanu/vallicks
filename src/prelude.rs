#[allow(unused_imports)]
pub use crate::{
    println,
    print,
    sprintln,
    sprint,
    hlt_loop as halt,
};

#[allow(unused_imports)]
pub use crate::schedule as thread;

// TODO: Make this exit thread based instead
#[allow(unused_imports)]
pub mod exit {
    use crate::exit;
    use crate::ExitCode;
}

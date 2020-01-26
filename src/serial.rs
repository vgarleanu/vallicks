use core::fmt::Write;
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut port = unsafe { SerialPort::new(0x3f8) };
        port.init();
        Mutex::new(port)
    };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed")
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! sprint {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! sprintln {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::sprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::sprint!(
        concat!($fmt, "\n"), $($arg)*));
}

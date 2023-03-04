use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

const PORT_COM1: u16 = 0x3f8;

lazy_static! {
    pub static ref COM1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(PORT_COM1) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

pub unsafe fn init_serial() {
    serial_println!("rigoros connected");
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    COM1.lock().write_fmt(args).ok();
}

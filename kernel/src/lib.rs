#![no_std]

mod terminal;

use core::fmt::Write;
use core::panic::PanicInfo;
use terminal::TERM;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    for i in 0..99 {
        writeln!(TERM.lock(), "fucking rust {}", i).unwrap();
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

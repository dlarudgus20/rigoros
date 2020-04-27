#![no_std]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    term_write_string("Hello Rust ; Fuck M$");
    loop {}
}

fn term_write_string(s: &str) {
    let video = 0xffff80001feb8000 as *mut u16;
    let len = if s.len() < 80 * 1 { s.len() } else { 80 * 1 };
    for i in 0..len {
        unsafe {
            *video.add(i) = 0x0700 | s.as_bytes()[i] as u16;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


#![no_std]

#![feature(asm)]

mod terminal;

use core::panic::PanicInfo;
use terminal::Terminal;
use terminal::ColorCode;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    let mut term = Terminal::create();
    term.write_string(ColorCode::DEFAULT, "fucking rust 00\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 01\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 02\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 03\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 04\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 05\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 06\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 07\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 08\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 09\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 10\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 11\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 12\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 13\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 14\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 15\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 16\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 17\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 18\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 19\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 20\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 21\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 22\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 23\n");
    term.write_string(ColorCode::DEFAULT, "fucking rust 24");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


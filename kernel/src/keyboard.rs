use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::instructions::port::Port;
use pc_keyboard::{layouts, KeyCode, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::{print, print_status};
use crate::terminal;
use crate::pic::{Irq, send_eoi};
use crate::interrupt_queue::{InterruptMessage, intmsg_push};

#[allow(dead_code)]
const KB_PORT_CTRL: u16 = 0x64;
const KB_PORT_DATA: u16 = 0x60;

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

pub fn init_keyboard() {
}

pub fn keyboard_handler(data: u8) {
    let mut keyboard = KEYBOARD.lock();

    fn print_line_status() {
        let terminal::LineInfo {
            screen, height, total, ..
        } = terminal::line_info();

        let scr_page = (screen + height - 1) / height;
        let scr_reminder = screen % height;
        let total_page =
            (total - scr_reminder + height - 1) / height
            + if scr_reminder > 0 { 1 } else { 0 };

        print_status!("page {} / {}, line {} / {}", scr_page + 1, total_page, screen + 1, total);
    }

    fn print_cursor_status() {
        let terminal::LineInfo {
            cur_col, cur_row, width, total, ..
        } = terminal::line_info();

        print_status!("row {} / {}, col {} / {}", cur_row + 1, total, cur_col + 1, width);
    }

    if let Ok(Some(evt)) = keyboard.add_byte(data) {
        if let Some(key) = keyboard.process_keyevent(evt) {
            match key {
                DecodedKey::RawKey(KeyCode::PageUp) => {
                    terminal::scroll(-1);
                    print_line_status();
                }
                DecodedKey::RawKey(KeyCode::PageDown) => {
                    terminal::scroll(1);
                    print_line_status();
                }
                DecodedKey::Unicode(ch) => {
                    print!("{}", ch);
                    print_cursor_status();
                }
                DecodedKey::RawKey(key) => {
                    print!("{:?}", key);
                    print_cursor_status();
                }
            }
        }
    }
}

pub extern "x86-interrupt" fn keyboard_int_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        send_eoi(Irq::KEYBOARD);
    }

    let mut port = Port::new(KB_PORT_DATA);
    let data = unsafe { port.read() };
    intmsg_push(InterruptMessage::Keyboard(data));
}

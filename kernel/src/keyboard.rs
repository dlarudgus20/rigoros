use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::instructions::port::Port;
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};

use crate::terminal;
use crate::pic::{Irq, send_eoi};
use crate::interrupt_queue::{InterruptMessage, intmsg_push};

#[allow(dead_code)]
const KB_PORT_CTRL: u16 = 0x64;
const KB_PORT_DATA: u16 = 0x60;

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore));
}

pub unsafe fn init_keyboard() {
}

pub fn keyboard_handler(data: u8) {
    let mut keyboard = KEYBOARD.lock();

    if let Ok(Some(evt)) = keyboard.add_byte(data) {
        if let Some(key) = keyboard.process_keyevent(evt) {
            terminal::process_input(key);
        }
    }
}

pub extern "x86-interrupt" fn keyboard_int_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(KB_PORT_DATA);
    let data = unsafe { port.read() };

    intmsg_push(InterruptMessage::Keyboard(data));

    unsafe {
        send_eoi(Irq::KEYBOARD);
    }
}

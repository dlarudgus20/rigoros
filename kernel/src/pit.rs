use x86_64::structures::idt::InterruptStackFrame;

use crate::pic;
use crate::print;

pub fn init_pit() {
}

pub extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        pic::send_eoi(pic::Irq::TIMER);
    }

    print!(".");
}

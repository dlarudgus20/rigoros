use x86_64::structures::idt::InterruptStackFrame;

use crate::pic::{Irq, send_eoi};
use crate::interrupt_queue::{InterruptMessage, intmsg_push};

pub fn init_pit() {
}

pub fn timer_handler() {
}

pub extern "x86-interrupt" fn timer_int_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        send_eoi(Irq::TIMER);
    }

    intmsg_push(InterruptMessage::Timer());
}

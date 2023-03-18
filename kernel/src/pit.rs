use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::pic::{Irq, send_eoi};
use crate::interrupt_queue::{InterruptMessage, intmsg_push};

const TIMER_FREQ: u32 = 1000;
const PIT_FREQ: u32 = 1193180;

const PIT_PORT_CTRL: u16 = 0x43;
const PIT_PORT_CNT0: u16 = 0x40;

const PIT_CTRL_CNT0: u8 = 0x00;
const PIT_CTRL_LSBMSBRW: u8 = 0x00;
const PIT_CTRL_MODE0: u8 = 0x00;
const PIT_CTRL_BINARY: u8 = 0x00;

static TICK_COUNTER: AtomicU64 = AtomicU64::new(0);

pub unsafe fn init_pit() {
    let mut ctrl = Port::new(PIT_PORT_CTRL);
    let mut cnt0 = Port::new(PIT_PORT_CNT0);

    let count = (PIT_FREQ / TIMER_FREQ) as u16;

    unsafe {
        ctrl.write(PIT_CTRL_CNT0 | PIT_CTRL_LSBMSBRW | PIT_CTRL_MODE0 | PIT_CTRL_BINARY);
        cnt0.write(count as u8);
        cnt0.write((count >> 8) as u8);
    }
}

pub fn tick() -> u64 {
    return TICK_COUNTER.load(Ordering::SeqCst);
}

pub fn timer_handler() {
    TICK_COUNTER.fetch_add(1, Ordering::SeqCst);
}

pub extern "x86-interrupt" fn timer_int_handler(_stack_frame: InterruptStackFrame) {
    intmsg_push(InterruptMessage::Timer());

    unsafe {
        send_eoi(Irq::TIMER);
    }
}

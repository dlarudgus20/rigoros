use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::ring_buffer::RingBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptMessage {
    Timer(),
    Keyboard(u8),
}

const BUFFER_SIZE: usize = 4096;

lazy_static! {
    static ref QUEUE: Mutex<RingBuffer<'static, InterruptMessage>> = unsafe {
        const EMPTY: InterruptMessage = InterruptMessage::Timer();
        static mut BUFFER: [InterruptMessage; BUFFER_SIZE] = [EMPTY; BUFFER_SIZE];
        Mutex::new(RingBuffer::new(&mut BUFFER))
    };
}

pub fn intmsg_push(msg: InterruptMessage) {
    without_interrupts(|| {
        let mut queue = QUEUE.lock();
        if queue.len() < BUFFER_SIZE {
            queue.try_push(msg).ok();
        }
    })
}

pub fn intmsg_pop() -> Result<InterruptMessage, ()> {
    without_interrupts(|| {
        QUEUE.lock().try_pop()
    })
}

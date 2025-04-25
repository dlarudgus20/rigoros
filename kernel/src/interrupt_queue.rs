use lazy_static::lazy_static;

use crate::irq_mutex::IrqMutex;
use crate::ring_buffer::RingBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptMessage {
    Timer(),
    Keyboard(u8),
}

const BUFFER_SIZE: usize = 4096;

lazy_static! {
    static ref QUEUE: IrqMutex<RingBuffer<InterruptMessage, BUFFER_SIZE>> = {
        const EMPTY: InterruptMessage = InterruptMessage::Timer();
        IrqMutex::new(RingBuffer::new_with(EMPTY))
    };
}

pub fn intmsg_push(msg: InterruptMessage) {
    let mut queue = QUEUE.lock();
    if queue.len() < BUFFER_SIZE {
        queue.try_push(msg);
    }
}

pub fn intmsg_pop() -> Option<InterruptMessage> {
    QUEUE.lock().try_pop()
}

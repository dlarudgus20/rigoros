#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]

pub mod terminal;
pub mod idt;
pub mod gdt;
pub mod pic;
pub mod interrupt_queue;
pub mod pit;
pub mod keyboard;
pub mod ring_buffer;

use core::panic::PanicInfo;
use crate::interrupt_queue::{InterruptMessage, intmsg_pop};

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    println!("terminal initialized");

    gdt::init_gdt();
    println!("gdt initialized");

    idt::init_idt();
    println!("idt initialized");

    pic::init_pic();
    println!("pic initialized");

    pit::init_pit();
    println!("pit initialized");

    keyboard::init_keyboard();
    println!("keyboard initialized");

    unsafe {
        pic::set_mask(pic::Mask::TIMER | pic::Mask::KEYBOARD | pic::Mask::SLAVE);
    }
    x86_64::instructions::interrupts::enable();
    println!("interrupt enabled");

    println!("done");

    loop {
        match intmsg_pop() {
            Ok(InterruptMessage::Timer()) => pit::timer_handler(),
            Ok(InterruptMessage::Keyboard(data)) => keyboard::keyboard_handler(data),
            _ => x86_64::instructions::hlt(),
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("panic!!");
    halt_loop();
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

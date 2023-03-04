#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]

pub mod fixed_writer;
pub mod irq_mutex;
pub mod serial;
pub mod terminal;
pub mod idt;
pub mod gdt;
pub mod pic;
pub mod interrupt_queue;
pub mod pit;
pub mod keyboard;
pub mod ring_buffer;

use crate::interrupt_queue::{InterruptMessage, intmsg_pop};

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    unsafe {
        serial::init_serial();

        terminal::init_term();
        terminal::set_status_lines_back(1);
        log!("terminal initialized");

        gdt::init_gdt();
        log!("gdt initialized");

        idt::init_idt();
        log!("idt initialized");

        pic::init_pic();
        log!("pic initialized");

        pit::init_pit();
        log!("pit initialized");

        keyboard::init_keyboard();
        log!("keyboard initialized");

        pic::set_mask(pic::Mask::TIMER | pic::Mask::KEYBOARD | pic::Mask::SLAVE);
        x86_64::instructions::interrupts::enable();
        log!("interrupt enabled");
    }

    log!("done");

    let mut buffer = [0u8; terminal::INPUT_MAXSIZE];
    print!("> ");
    terminal::start_inputting();

    loop {
        match intmsg_pop() {
            Ok(InterruptMessage::Timer()) => pit::timer_handler(),
            Ok(InterruptMessage::Keyboard(data)) => keyboard::keyboard_handler(data),
            _ => x86_64::instructions::hlt(),
        }

        if let Ok(input) = terminal::getline(&mut buffer) {
            println!("input: {}", input);
        }
    }
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

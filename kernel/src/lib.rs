#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]

pub mod terminal;
pub mod idt;
pub mod gdt;
pub mod pic;
pub mod pit;

use core::panic::PanicInfo;

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

    unsafe {
        pic::set_mask(pic::Mask::TIMER | pic::Mask::SLAVE);
    }
    x86_64::instructions::interrupts::enable();
    println!("interrupt enabled");

    println!("done");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("panic!!");
    loop {}
}

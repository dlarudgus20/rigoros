#![no_std]
#![feature(abi_x86_interrupt)]

mod terminal;
mod interrupts;
mod gdt;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    println!("terminal initialized");

    gdt::init_gdt();
    println!("gdt initialized");

    interrupts::init_idt();
    println!("idt initialized");

    unsafe { core::arch::asm!("int3"); }

    println!("done");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("panic!!");
    loop {}
}

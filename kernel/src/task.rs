use x86_64::VirtAddr;
use x86_64::registers::rflags;

use crate::println;
use crate::context::{Context, switch_context};
use crate::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};

#[allow(dead_code)]
pub struct Task {
    context: Context,
    stack: VirtAddr,
    stack_size: usize,
}

impl Task {
    pub fn new() {

    }
}

pub fn test_task() {
    use spin::Once;
    
    unsafe {
        static mut CURRENT: Context = Context::new();
        static mut TASK: Context = Context::new();

        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            TASK.rip = task_main as u64;
            TASK.cs = KERNEL_CODE_SELECTOR.into();

            TASK.rflags = rflags::read_raw();

            TASK.rsp = 0x400000;
            TASK.rbp = TASK.rsp;

            TASK.ss = KERNEL_DATA_SELECTOR.into();
            TASK.ds = KERNEL_DATA_SELECTOR.into();
            TASK.es = KERNEL_DATA_SELECTOR.into();
            TASK.fs = KERNEL_DATA_SELECTOR.into();
            TASK.gs = KERNEL_DATA_SELECTOR.into();

            TASK.rdi = 42;
        });

        unsafe extern "C" fn task_main(arg: u64) {
            static mut COUNT: u32 = 0;
            println!("hello task(arg={})", arg);
            loop {
                println!("task loop #{}, rsp={:#x}", COUNT, TASK.rsp);
                unsafe {
                    COUNT += 1;
                    switch_context(&mut TASK, &CURRENT);
                }
            }
        }

        switch_context(&mut CURRENT, &TASK);
    }
}

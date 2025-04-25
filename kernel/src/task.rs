use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::registers::rflags;

use crate::println;
use crate::irq_mutex::IrqMutex;
use crate::context::{Context, switch_context};
use crate::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};

#[allow(dead_code)]
pub struct Task {
    context: Context,
    stack: VirtAddr,
    stack_size: usize,
}

pub struct Scheduler {

}

lazy_static! {
    static ref SCHEDULER: IrqMutex<Scheduler> = IrqMutex::new(Scheduler {

    });
}

impl Task {
    pub fn new() {

    }
}

pub fn test_task(quit: bool) {
    use core::mem::size_of;
    use spin::Mutex;
    use crate::memory::{alloc_zero, deallocate};

    struct CtxData {
        parameter: u64,
        this: Context,
        main: Context,
        stack: [u8; 8192],
    }

    let parameter = 42;

    lazy_static! {
        static ref CTX_PTR: Mutex<usize> = Mutex::new(0);
    }

    let mut ctx_ptr = CTX_PTR.lock();

    if quit {
        if *ctx_ptr != 0 {
            deallocate(*ctx_ptr, size_of::<CtxData>());
            *ctx_ptr = 0;
        }
    }
    else {
        if *ctx_ptr == 0 {
            let data_raw = alloc_zero(size_of::<CtxData>()).unwrap();
            let data = unsafe { &mut *(data_raw as *mut CtxData) };

            data.this.rip = task_main as u64;
            data.this.cs = KERNEL_CODE_SELECTOR.into();

            data.this.rflags = rflags::read_raw();

            data.this.rsp = data.stack.as_ptr_range().end as u64;
            data.this.rbp = data.this.rsp;

            data.this.ss = KERNEL_DATA_SELECTOR.into();
            data.this.ds = KERNEL_DATA_SELECTOR.into();
            data.this.es = KERNEL_DATA_SELECTOR.into();
            data.this.fs = KERNEL_DATA_SELECTOR.into();
            data.this.gs = KERNEL_DATA_SELECTOR.into();

            data.this.rdi = data_raw as u64;

            data.parameter = parameter;

            *ctx_ptr = data_raw;
        }

        let data = unsafe { &mut *(*ctx_ptr as *mut CtxData) };

        unsafe {
            switch_context(&mut data.main, &data.this);
        }
    }

    unsafe extern "C" fn task_main(arg: u64) {
        let data = unsafe { &mut *(arg as *mut CtxData) };
        println!("hello task(parameter={})", data.parameter);
        let mut count = 1;
        loop {
            println!("task loop #{}, rsp={:#x}", count, data.this.rsp);
            count += 1;
            unsafe {
                switch_context(&mut data.this, &data.main);
            }
        }
    }
}

use core::fmt;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::println;
use crate::gdt;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // exceptions
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
        idt.vmm_communication_exception.set_handler_fn(vmm_communication_exception_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);

        for i in 32..256 {
            idt[i].set_handler_fn(unknown_handler);
        }

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    println!("#DE {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("#DB {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    println!("#NMI {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("#BP {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("#OF {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    println!("#BR {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    println!("#UD {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    println!("#NM {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    println!("#DF:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#TS:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#NP:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#SS:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#GP:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    println!("#PF:{} {}", PFCode(error_code), StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!("#MF {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#AC:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    println!("#MC {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!("#XF {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    println!("#VE {}", StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn vmm_communication_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#VC:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("#SX:{:#018x} {}", error_code, StackFrame(stack_frame));
    loop {}
}

extern "x86-interrupt" fn unknown_handler(stack_frame: InterruptStackFrame) {
    println!("#UNKNOWN {}", StackFrame(stack_frame));
    loop {}
}

struct StackFrame(InterruptStackFrame);
struct PFCode(PageFaultErrorCode);

impl fmt::Display for StackFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let value = *self.0;
        write!(formatter, "cs:ip={:#06x}:{:#018x}, ss:sp={:#06x}:{:#018x}, rflags={:#018x}",
            value.code_segment, value.instruction_pointer.as_u64(), value.stack_segment, value.stack_pointer.as_u64(), value.cpu_flags)
    }
}

impl fmt::Display for PFCode {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let p = match self.0 { PageFaultErrorCode::PROTECTION_VIOLATION => "P", _ => "p" };
        let w = match self.0 { PageFaultErrorCode::CAUSED_BY_WRITE => "W", _ => "w" };
        let u = match self.0 { PageFaultErrorCode::USER_MODE => "U", _ => "u" };
        let r = match self.0 { PageFaultErrorCode::MALFORMED_TABLE => "R", _ => "r" };
        let i = match self.0 { PageFaultErrorCode::INSTRUCTION_FETCH => "I", _ => "i" };
        let k = match self.0 { PageFaultErrorCode::PROTECTION_KEY => "K", _ => "k" };
        let s = match self.0 { PageFaultErrorCode::SHADOW_STACK => "S", _ => "s" };
        let g = match self.0 { PageFaultErrorCode::SGX => "G", _ => "g" };
        let m = match self.0 { PageFaultErrorCode::RMP => "M", _ => "m" };
        write!(formatter, "{:#018x} {}{}{}{}{}{}{}{}{}", self.0.bits(), m, g, s, k, i, r, u, w, p)
    }
}

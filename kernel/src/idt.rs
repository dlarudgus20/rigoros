use core::fmt;
use lazy_static::lazy_static;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::gdt;
use crate::pic::Irq;
use crate::pit::timer_int_handler;
use crate::keyboard::keyboard_int_handler;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // exceptions
        idt.divide_error.set_handler_fn(divide_error_int_handler);
        idt.debug.set_handler_fn(debug_int_handler);
        idt.non_maskable_interrupt.set_handler_fn(nmi_int_handler);
        idt.breakpoint.set_handler_fn(breakpoint_int_handler);
        idt.overflow.set_handler_fn(overflow_int_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_int_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_int_handler);
        idt.device_not_available.set_handler_fn(device_not_available_int_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_int_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.invalid_tss.set_handler_fn(invalid_tss_int_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_int_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_int_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_int_handler);
        idt.page_fault.set_handler_fn(page_fault_int_handler);
        idt.x87_floating_point.set_handler_fn(x87_floating_point_int_handler);
        idt.alignment_check.set_handler_fn(alignment_check_int_handler);
        idt.machine_check.set_handler_fn(machine_check_int_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_int_handler);
        idt.virtualization.set_handler_fn(virtualization_int_handler);
        idt.vmm_communication_exception.set_handler_fn(vmm_communication_exception_int_handler);
        idt.security_exception.set_handler_fn(security_exception_int_handler);

        // unknown
        for i in 32..256 {
            idt[i].set_handler_fn(unknown_int_handler);
        }

        // pic
        idt[Irq::TIMER.as_intn()].set_handler_fn(timer_int_handler);
        idt[Irq::KEYBOARD.as_intn()].set_handler_fn(keyboard_int_handler);

        idt
    };
}

pub unsafe fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn divide_error_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#DE {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn debug_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#DB {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn nmi_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#NMI {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn breakpoint_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#BP {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn overflow_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#OF {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn bound_range_exceeded_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#BR {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn invalid_opcode_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#UD {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn device_not_available_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#NM {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn double_fault_int_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    panic!("#DF:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn invalid_tss_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#TS:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn segment_not_present_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#NP:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn stack_segment_fault_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#SS:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn general_protection_fault_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#GP:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn page_fault_int_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    panic!("#PF:{} access={:#018x} {}", PFCode(error_code), Cr2::read_raw(), StackFrame(stack_frame));
}

extern "x86-interrupt" fn x87_floating_point_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#MF {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn alignment_check_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#AC:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn machine_check_int_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!("#MC {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn simd_floating_point_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#XF {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn virtualization_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#VE {}", StackFrame(stack_frame));
}

extern "x86-interrupt" fn vmm_communication_exception_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#VC:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn security_exception_int_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("#SX:{:#018x} {}", error_code, StackFrame(stack_frame));
}

extern "x86-interrupt" fn unknown_int_handler(stack_frame: InterruptStackFrame) {
    panic!("#UNKNOWN {}", StackFrame(stack_frame));
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

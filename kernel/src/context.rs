#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Context {
    pub gs: u64,
    pub fs: u64,
    pub es: u64,
    pub ds: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rbp: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

extern "C" {
    pub fn switch_context(from: &mut Context, to: &Context);
}

impl Context {
    pub const fn new() -> Self {
        Self {
            gs: 0, fs: 0, es: 0, ds: 0,
            r15: 0, r14: 0, r13: 0, r12: 0, r11: 0, r10: 0, r9: 0, r8: 0,
            rsi: 0, rdi: 0, rdx: 0, rcx: 0, rbx: 0, rax: 0, rbp: 0,
            rip: 0, cs: 0, rflags: 0, rsp: 0, ss: 0,
        }
    }
}

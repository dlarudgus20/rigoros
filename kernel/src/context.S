.section .text
.code64

// fn switch_context(from: &mut Context, to: &Context)
.global switch_context
switch_context:
    push %rbp
    mov %rsp, %rbp

    push %rax
    pushfq
    cli

    xor %rax, %rax
    mov %ss, %ax
    mov %rax, 23 * 8(%rdi)  // ss

    mov %rbp, %rax
    add $16, %rax           // push %rbp + return address
    mov %rax, 22 * 8(%rdi)  // rsp

    pop %rax
    mov %rax, 21 * 8(%rdi)  // rflags

    xor %rax, %rax
    mov %cs, %ax
    mov %rax, 20 * 8(%rdi)  // cs

    mov 8(%rbp), %rax       // return address
    mov %rax, 19 * 8(%rdi)  // rip

    pop %rax
    pop %rbp

    // save other registers
    mov %rdi, %rsp
    add $(19 * 8), %rsp
    push %rbp
    push %rax
    push %rbx
    push %rcx
    push %rdx
    push %rdi
    push %rsi
    push %r8
    push %r9
    push %r10
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15
    xor %rax, %rax
    mov %ds, %ax
    push %rax
    mov %es, %ax
    push %rax
    push %fs
    push %gs

    // restore context
    mov %rsi, %rsp
    pop %gs
    pop %fs
    pop %rax
    mov %ax, %es
    pop %rax
    mov %ax, %ds
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %r11
    pop %r10
    pop %r9
    pop %r8
    pop %rsi
    pop %rdi
    pop %rdx
    pop %rcx
    pop %rbx
    pop %rax
    pop %rbp
    iretq

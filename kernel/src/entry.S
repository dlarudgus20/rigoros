.extern kmain
.global _entry

.section .text
.code64

.extern __page_table
.extern __stack_top

_entry:
    // clear temporary page to lower 2MB
    movabs $__page_table, %rax
    movq $0, (%rax)
    mov %cr3, %rax
    mov %rax, %cr3

    movabs $__stack_top, %rsp
    mov %rsp, %rbp
    cld

    call kmain

0:
    hlt
    jmp 0b

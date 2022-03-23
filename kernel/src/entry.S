.extern kmain
.global _entry

.section .text
.code64

.set PAGE_TABLE, 0xffff8000003f0000
.set STACK_TOP, 0xffff80000f200000

_entry:
    // clear temporary page to lower 2MB
    movabs $PAGE_TABLE, %rax
    movq $0, (%rax)
    mov %cr3, %rax
    mov %rax, %cr3

    movabs $STACK_TOP, %rsp
    mov %rsp, %rbp
    cld

    call kmain

0:
    hlt
    jmp 0b

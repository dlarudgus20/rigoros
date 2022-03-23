.extern __kernel_lma_start, __kernel_start, __kernel_size
.extern __bss_start, __bss_size
.extern _entry

.set PAGE_TABLE, 0x005f0000
.set MEMORY_MAP, 0x6000

.section .startup, "ax", @progbits

.code16
startup:
    cli

    // enable a20 gate
    call a20_check
    test %ax, %ax
    jnz 1f

    mov $0x2401, %ax
    int $0x15
    jc 0f
    test $0, %ax
    jnz 0f

    call a20_check
    test %ax, %ax
    jnz 1f
0:
    mov $a20_failmsg, %si
    jmp panic16
1:

    // memory detection
    xor %bp, %bp            // counter variable
    xor %ebx, %ebx          // initially ebx=0
    mov %bx, %es            // es:di -> buffer
    mov $MEMORY_MAP + 8, %di
0:
    movl $1, %es:20(%di)    // allow ACPI 3.x entry
    mov $0x534d4150, %edx   // magic number
    mov $24, %ecx           // request 24 bytes
    mov $0xe820, %eax
    int $0x15               // ebx <- some internal value
    mov $a20_failmsg, %si   // cl <- stored bytes
    jc panic16
    cmp $0x534d4150, %eax
    jne panic16
1:
    jcxz 3f                 // ignore if bytes=0
    cmp $20, %cl            // or "ignore" ACPI 3.x attribute is 1
    jbe 2f
    testb $1, %es:20(%di)
    jz 3f
2:
    mov %es:8(%di), %ecx    // ignore if length=0
    or %es:12(%di), %ecx
    jz 3f
    inc %bp
    add $24, %di
3:
    test %ebx, %ebx         // ebx=0 if end of list
    jnz 0b
    mov %bp, (MEMORY_MAP)
    clc

    // enter protected mode
    lgdt gdtr
    mov %cr0, %eax
    or $0x00000001, %eax
    and $0x7fffffff, %eax
    mov %eax, %cr0
    jmp .+2
    ljmp $0x18, $pm_start

// ax=0 if a20 is disabled; ax=1 if a20 is enabled
a20_check:
    push %ds
    push %es
    push %si
    push %di

    xor %ax, %ax
    mov %ax, %es // es := 0x0000
    not %ax
    mov %ax, %ds // ds := 0xffff
    mov $0x0500, %di
    mov $0x0510, %si

    mov %es:(%di), %dh
    mov %ds:(%si), %dl
    movb $0x00, %es:(%di)
    movb $0xff, %ds:(%si)
    cmpb $0xff, %es:(%di)

    mov %dh, %es:(%di)
    mov %dl, %ds:(%si)
    mov $0, %ax
    je 0f
    mov $1, %ax
0:
    pop %di
    pop %si
    pop %es
    pop %ds
    ret

panic16:
    cld
0:
    lodsb
    test %al, %al
    jz 1f
    mov $0x07, %bx
    mov $0x0e, %ah
    int $0x10
    jmp 0b
1:
    hlt
    jmp 1b

.code32
pm_start:
    mov $0x20, %eax
    mov %eax, %ds
    mov %eax, %es
    mov %eax, %fs
    mov %eax, %gs
    mov %eax, %ss
    mov $0x7c00, %esp
    cld

    // clear screen
    mov $0xb8000, %edi
    mov $1000, %ecx
    mov $0x07000700, %eax
    repne stosl

    // check if cpuid is suppported
    pushfl
    pop %eax
    mov %eax, %ecx
    xor $1 << 21, %eax
    push %eax
    popfl
    pushfl
    pop %eax
    push %ecx
    popfl
    xor %ecx, %eax
    jz panic32

    // check if long mode is supported
    mov $0x80000000, %eax
    cpuid
    cmp $0x80000001, %eax
    jb panic32
    mov $0x80000001, %eax
    cpuid
    test $1 << 29, %edx
    jz panic32

    // set up paging
    mov $PAGE_TABLE, %ebx
    mov %ebx, %edi
    mov $0x4000, %ecx
    xor %eax, %eax
    rep stosl

    // PML4
    movl $0x1003, (%ebx)
    add $0x800, %ebx
    movl $0x005f1003, (%ebx)
    add $0x800, %ebx
    // PDPT
    movl $0x005f2003, (%ebx)
    add $0x1000, %ebx

    // page directory for kernel code/bss
    mov %ebx, %edi
    add $0x1000, %edi
    mov $2, %ecx
    mov $0x00200000, %eax
    call page_init
    add $0x2000, %edi

    // page directory for kernel stack
    add $0x78 * 8, %ebx             // 0x78 directory forward (+0x0f000000)
    mov $0x00600000, %eax           //  -> 0xffff8000 0f000000
    mov $1, %ecx
    call page_init
    add $0x1000, %edi

    // page directory for lower 2MB memory
    add $0x87 * 8, %ebx             // 0x87 directory forward (+0x10e00000)
    xor %eax, %eax                  //  -> 0xffff8000 1fe00000
    mov $1, %ecx
    call page_init

    // temporary page for lower 1MB
    mov $0x1000, %ebx
    movl $0x2003, (%ebx)
    mov $0x2000, %ebx
    mov $0x3000, %edi
    xor %eax, %eax
    mov $1, %ecx
    call page_init

    mov $PAGE_TABLE, %eax
    mov %eax, %cr3

    mov %cr4, %eax
    or $1 << 5, %eax
    mov %eax, %cr4

    // enter long mode
    mov $0xc0000080, %ecx
    rdmsr
    or $1 << 8, %eax
    wrmsr

    mov %cr0, %eax
    or $1 << 31, %eax
    mov %eax, %cr0

    ljmp $0x08, $lm_start

// initialize one or more page directories
// ebx=start address of page directory
// edi=address of page table
// ecx=count of page directory to create
// eax=target physical address of page
page_init:
    push %ebx
    push %ecx
    push %edi
0:
    mov %edi, %edx
    or $3, %edx
    mov %edx, (%ebx)
    movl $0, 4(%ebx)
1:
    mov %eax, %edx
    or $3, %edx
    mov %edx, (%edi)
    movl $0, 4(%edi)
    add $0x1000, %eax
    add $8, %edi
    test $0xfff, %edi
    jnz 1b
    add $8, %ebx
    dec %ecx
    jnz 0b
    pop %edi
    pop %ecx
    pop %ebx
    ret

panic32:
    mov $lm_failmsg, %esi
    mov $0xb8000, %edi
    mov $0x07, %ah
    cld
0:
    lodsb
    test %al, %al
    jz 1f
    stosw
    jmp 0b
1:
    hlt
    jmp 1b

.code64
lm_start:
    mov $0x10, %rax
    mov %rax, %ds
    mov %rax, %es
    mov %rax, %fs
    mov %rax, %gs
    mov %rax, %ss
    mov $0x7c00, %rsp
    mov %rsp, %rbp
    cld

    // move kernel to 0x00200000
    mov $__kernel_lma_start, %rsi
    movabs $__kernel_start, %rdi
    mov $__kernel_size, %rcx
    rep movsb

    movabs $__bss_start, %rdi
    mov $__bss_size, %rcx
    xor %rax, %rax
    rep stosb

    movabs $_entry, %rax
    jmp *%rax

gdtr:
    .word gdt_end - gdt - 1
    .long gdt
gdt:
    .long 0, 0
    // 64bit code segment
    .word 0xffff
    .word 0x0000
    .byte 0x00
    .byte 0b10011010
    .byte 0b10101111
    .byte 0x00
    // 64bit data segment
    .word 0xffff
    .word 0x0000
    .byte 0x00
    .byte 0b10010010
    .byte 0b10101111
    .byte 0x00
    // 32bit code segment
    .word 0xffff
    .word 0x0000
    .byte 0x00
    .byte 0b10011010
    .byte 0b11001111
    .byte 0x00
    // 32bit data segment
    .word 0xffff
    .word 0x0000
    .byte 0x00
    .byte 0b10010010
    .byte 0b11001111
    .byte 0x00
gdt_end:

a20_failmsg: .asciz "Cannot enable A20 gate"
mem_failmsg: .asciz "Cannot detect memory"
lm_failmsg: .asciz "This computer does not support 64bit mode"

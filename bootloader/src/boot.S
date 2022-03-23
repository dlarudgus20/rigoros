#ifndef STAGE_SECTORS
#error STAGE_SECTORS is not defined
#endif

.section .boot, "awx", @progbits
.code16

.set stage_start, 0x8000

// FAT BPB
    jmp start
.byte 0x90              // nop
.ascii "STARRIOS"       // OEM identifier (8byte)
.word 512               // bytes per sector
.byte 1                 // sectors per cluster
.word 1                 // reserved sectors
.byte 2                 // number of FATs
.word 224               // number of root directory entries
.word 2880              // number of total sectors
.byte 0xf0              // media descriptor type (0xf0=1.44MB/2.88MB floppy)
.word 9                 // sectors per FAT
.word 18                // sectors per track
.word 2                 // number of heads
.long 0                 // number of hidden sectors
.long 0                 // number of total sectors (32bit)
// FAT12/16 Extended
.byte 0x00              // drive number (floppy: 0x00, hard: 0x80)
.byte 0                 // reserved
.byte 0x29              // signature
.long 0xffffffff        // VolumeID Serial Number
.ascii "STARRIOS-FD"    // volume label string (11byte)
.ascii "FAT12   "       // file system type (8byte)
.fill 28, 1, 0          // FAT32 takes 28 bytes more

start:
    ljmp $0, $1f
1:
    cli
    xor %ax, %ax
    mov %ax, %ds
    mov %ax, %ss
    mov $0x7c00, %sp
    mov %sp, %bp
    mov %dl, drive
    sti

    // check LBA extension
    mov $0x41, %ah
    mov $0x55aa, %bx
    int $0x13
    jc nolba
    cmp $0xaa55, %bx
    jne nolba
    test $1, %cl
    jz nolba

    // int 0x13, ah=0x42 extended (LBA) read sector
    // read 127 sectors at once
    mov stage_sectors, %si
0:
    mov %si, %dx
    sub $127, %si
    jle 1f
    mov $127, %dx
1:
    xor %eax, %eax
    mov stage_lba, %ebx
    push %eax
    push %ebx
    push %ax
    push $stage_start
    push stage_sectors
    push $16
    mov %sp, %si
    mov drive, %dl
    mov $0x42, %ah
    int $0x13
    jc panic

    cmp $0, %si
    jg 0b

    jmp done

nolba:
    // int 0x13, ah=0x08 get drive parameters
    mov $0x08, %ah
    mov drive, %dl
    xor %di, %di
    mov %di, %es
    int $0x13
    jc floppy
    mov %ch, %dl
    mov %dh, %ch
    push %dx
    push %cx
    jmp 0f
floppy:
    mov drive, %al
    test $0xfe, %al
    jnz panic
    push $79
    push $0x0112
0:

    // int 0x13, ah=0x02 (CHS) read sector
    xor %ax, %ax
    mov %ax, %es
    mov $stage_start, %bx
    mov chs.cylinders, %ch
    mov chs.heads, %dh
    mov chs.sectors, %cl
    mov drive, %dl
    mov stage_sectors, %di
0:
    mov $5, %si
1:
    mov $0x0201, %ax
    int $0x13
    jnc 2f
    dec %si
    jz panic
    xor %ax, %ax    // int 0x13, ah=0x00 reset disk
    int $0x13
    jmp 1b
2:
    mov %es, %ax
    add $0x20, %ax
    mov %ax, %es

    inc %cl
    movzx -4(%bp), %ax
    mov %ax, %si
    mov %cl, %ah
    and $0x3f3f, %ax
    cmp %al, %ah
    jbe 4f

    and $0xc0, %cl
    inc %cx
    inc %dh
    cmp -3(%bp), %dh
    jbe 4f

    xor %dh, %dh
    add $1, %ch
    jnc 3f
    add $0x40, %cl
3:
    mov %si, %ax
    shl $2, %ax
    mov -2(%bp), %al
    mov %ax, %si
    mov %ch, %al
    mov %cl, %ah
    shr $6, %ah
    cmp %si, %ax
    ja panic
4:
    dec %di
    jnz 0b

done:
    jmp stage_start

panic:
    mov $2f, %si
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
2: .asciz "boot panic"

.align 4, 0

drive: .byte 0
chs.sectors: .byte 16
chs.heads: .byte 1
chs.cylinders: .byte 0

stage_lba: .long 33

stage_sectors: .word STAGE_SECTORS

.org 440

// disk signature
.long 0
.word 0

// partition table
.quad 0, 0
.quad 0, 0
.quad 0, 0
.quad 0, 0

// boot signature
.byte 0x55, 0xaa

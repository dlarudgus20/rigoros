OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
    .startup 0x8000 :
    {
        KEEP(*(.startup))
        . = ALIGN(16);
    }

    __higher_half_displacement = 0xffff800000000000 - .;
    . = 0xffff800000000000;

    .text ALIGN(16) : AT(ADDR(.text) - __higher_half_displacement)
    {
        __kernel_start = .;
        KEEP(*(.text))
        *(.stub .text.* .gnu.linkonce.t.*)
    }
    __kernel_lma_start = __kernel_start - __higher_half_displacement;

    .rodata ALIGN(16) : AT(ADDR(.rodata) - __higher_half_displacement)
    {
        *(.rodata .rodata.* .gnu.linkonce.r.*)
        *(.rodata1)
    }

    .data ALIGN(16) : AT(ADDR(.data) - __higher_half_displacement)
    {
        *(.data .data.* .gnu.linkonce.d.*)
        *(.data1)
        . = ALIGN(16);
        __kernel_end = .;
    }
    __kernel_size = __kernel_end - __kernel_start;
    __kernel_lma_end = __kernel_lma_start + __kernel_size;

    __bss_displacement = 0xffff800000100000 - .;
    . = 0xffff800000100000;

    .bss ALIGN(16) : AT(ADDR(.bss) - __bss_displacement)
    {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.* .gnu.linkonce.b.*)
        . = ALIGN(16);
        __bss_end = .;
    }

    __bss_size = __bss_end - __bss_start;

    . = 0;

    .eh_frame : { KEEP(*(.eh_frame)) }

    .debug_info : { KEEP(*(.debug_info)) }
    .debug_macinfo : { KEEP(*(.debug_macinfo)) }
    .debug_ranges : { KEEP(*(.debug_ranges)) }
    .debug_aranges : { KEEP(*(.debug_aranges)) }
    .debug_pubnames : { KEEP(*(.debug_pubnames)) }
    .debug_pubtypes : { KEEP(*(.debug_pubtypes)) }
    .debug_abbrev : { KEEP(*(.debug_abbrev)) }
    .debug_line : { KEEP(*(.debug_line)) }
    .debug_frame : { KEEP(*(.debug_frame)) }
    .debug_str : { KEEP(*(.debug_str)) }
    .debug_loc : { KEEP(*(.debug_loc)) }
    .debug_gdb_scripts : { KEEP(*(.debug_gdb_scripts)) }
    .note.GNU-stack : { KEEP(*(.note.GNU-stack)) }

    /DISCARD/ :
    {
        *(.got .got.plt .igot .igot.plt .iplt)
        *(.rela .rela.*)
    }
}

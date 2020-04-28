OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
    .boot 0x7c00 :
    {
        KEEP(*(.boot))
    }

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

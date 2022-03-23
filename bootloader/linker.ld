OUTPUT_FORMAT(elf64-x86-64)

SECTIONS
{
    .boot 0x7c00 :
    {
        KEEP(*(.boot))
    }

    . = 0;
}

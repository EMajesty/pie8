ENTRY(_start)

SECTIONS
{
    . = 0x00080000;
    __image_start = .;

    .text : ALIGN(16) {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }
    .vectors : ALIGN(2048) {
        __vectors_start = .;
        KEEP(*(.vectors))
        __vectors_end = .;
    }
    .rodata : ALIGN(16) { *(.rodata .rodata.*) }
    .data : ALIGN(16) { *(.data .data.*) }

    .bss (NOLOAD) : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }

    .translation_tables (NOLOAD) : ALIGN(4096) {
        __translation_tables_start = .;
        KEEP(*(.translation_tables))
        __translation_tables_end = .;
    }

    .framebuffer (NOLOAD) : ALIGN(64) {
        __framebuffer_start = .;
        KEEP(*(.framebuffer))
        __framebuffer_end = .;
    }

    .stack (NOLOAD) : ALIGN(16) {
        __stack_bottom = .;
        . += 64K;
        __stack_top = .;
    }

    __image_end = .;
    /DISCARD/ : { *(.comment .note .note.* .eh_frame*) }
}

ASSERT(__image_start == 0x80000, "kernel must be linked at the Pi 64-bit load address");
ASSERT((__stack_top & 0xf) == 0, "AArch64 stack must be 16-byte aligned");
ASSERT((__vectors_start & 0x7ff) == 0, "VBAR_EL1 must be 2048-byte aligned");
ASSERT(__vectors_end - __vectors_start == 2048 + 12, "unexpected vector table size");
ASSERT(__framebuffer_end <= __stack_bottom, "framebuffer overlaps stack");
ASSERT((__translation_tables_start & 0xfff) == 0, "translation tables must be 4K aligned");
ASSERT(__translation_tables_end - __translation_tables_start == 8192, "expected two translation tables");
ASSERT(__image_end < 16M, "firmware exceeds the identity-mapped RAM window");

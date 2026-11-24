ENTRY(_start)

SECTIONS
{
    . = 0x00080000;
    __image_start = .;

    .text : ALIGN(16) {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }
    .rodata : ALIGN(16) { *(.rodata .rodata.*) }
    .data : ALIGN(16) { *(.data .data.*) }

    .bss (NOLOAD) : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(16);
        __bss_end = .;
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

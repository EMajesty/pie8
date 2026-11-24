.macro VECTOR_SLOT number
    mov x0, #\number
    b exception_common
    .balign 128
.endm

.section .vectors, "ax"
.balign 2048
.global exception_vectors
exception_vectors:
    // Current EL with SP0: sync, IRQ, FIQ, SError.
    VECTOR_SLOT 0
    VECTOR_SLOT 1
    VECTOR_SLOT 2
    VECTOR_SLOT 3
    // Current EL with SPx.
    VECTOR_SLOT 4
    VECTOR_SLOT 5
    VECTOR_SLOT 6
    VECTOR_SLOT 7
    // Lower EL using AArch64.
    VECTOR_SLOT 8
    VECTOR_SLOT 9
    VECTOR_SLOT 10
    VECTOR_SLOT 11
    // Lower EL using AArch32.
    VECTOR_SLOT 12
    VECTOR_SLOT 13
    VECTOR_SLOT 14
    VECTOR_SLOT 15

exception_common:
    bl exception_handler
1:
    wfe
    b 1b

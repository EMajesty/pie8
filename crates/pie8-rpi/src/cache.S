.section .text
.global invalidate_data_caches
invalidate_data_caches:
    mrs x0, clidr_el1
    ubfx x3, x0, #24, #3       // LoC: number of cache levels to clean.
    cbz x3, 3f
    mov x10, #0                // Cache level, zero based.
1:
    add x2, x10, x10, lsl #1  // CLIDR cache-type field shift = level * 3.
    lsr x1, x0, x2
    and x1, x1, #7
    cmp x1, #2                 // Skip instruction-only or absent levels.
    b.lt 2f
    lsl x2, x10, #1
    msr csselr_el1, x2
    isb
    mrs x1, ccsidr_el1
    and x2, x1, #7
    add x2, x2, #4            // Log2(bytes per cache line).
    ubfx x4, x1, #3, #10      // Maximum way number.
    clz w5, w4                // Way position in DC ISW operand.
    ubfx x6, x1, #13, #15     // Maximum set number.
4:
    mov x9, x6
5:
    lsl x7, x4, x5
    lsl x8, x9, x2
    orr x7, x7, x8
    lsl x8, x10, #1
    orr x7, x7, x8
    dc isw, x7
    subs x9, x9, #1
    b.ge 5b
    subs x4, x4, #1
    b.ge 4b
2:
    add x10, x10, #1
    cmp x10, x3
    b.lt 1b
3:
    msr csselr_el1, xzr
    dsb sy
    isb
    ret

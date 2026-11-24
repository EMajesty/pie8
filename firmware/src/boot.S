.section .text.boot
.global _start

// Raspberry Pi firmware can release every core at the kernel entry address.
// Inspect MPIDR before touching the one boot-core stack; secondary cores park.
_start:
    mrs x0, mpidr_el1
    and x0, x0, #0xff
    cbz x0, .Lprimary

.Lsecondary:
    wfe
    b .Lsecondary

.Lprimary:
    // Enter a known EL1h state whether firmware starts us at EL3, EL2, or EL1.
    mrs x0, CurrentEL
    cmp x0, #0xc
    b.eq .Lfrom_el3
    cmp x0, #0x8
    b.eq .Lfrom_el2
    cmp x0, #0x4
    b.eq .Lat_el1
    b .Lsecondary

.Lfrom_el3:
    // SCR_EL3.NS=1, HCE=1, RW=1: non-secure AArch64 below EL3.
    mov x0, #(1 << 0 | 1 << 8 | 1 << 10)
    msr scr_el3, x0
    msr cptr_el3, xzr           // Do not trap FP/SIMD at EL3.
    mov x0, #0x3c9              // Mask DAIF, return to EL2h.
    msr spsr_el3, x0
    adr x0, .Lfrom_el2
    msr elr_el3, x0
    eret

.Lfrom_el2:
    mov x0, #(1 << 31)          // HCR_EL2.RW: EL1 executes AArch64.
    msr hcr_el2, x0
    msr cptr_el2, xzr           // CPTR_EL2.TFP=0: do not trap EL1 FP/SIMD.
    mov x0, #3                  // Allow EL1 physical timer/counter access.
    msr cnthctl_el2, x0
    msr cntvoff_el2, xzr
    mov x0, #0x3c5              // Mask DAIF, return to EL1h.
    msr spsr_el2, x0
    adr x0, .Lat_el1
    msr elr_el2, x0
    eret

.Lat_el1:
    // MMU/caches off while preserving the Armv8-A SCTLR_EL1 RES1 fields.
    movz x0, #0x0800
    movk x0, #0x30d0, lsl #16
    msr sctlr_el1, x0
    mov x0, #(3 << 20)          // CPACR_EL1.FPEN=0b11: EL1 FP/SIMD enabled.
    msr cpacr_el1, x0
    ldr x0, =exception_vectors
    msr vbar_el1, x0
    isb
    ldr x0, =__stack_top
    mov sp, x0

    ldr x0, =__bss_start
    ldr x1, =__bss_end
.Lbss_loop:
    cmp x0, x1
    b.hs .Lbss_done
    str xzr, [x0], #8
    b .Lbss_loop
.Lbss_done:
    bl kernel_main
    b .Lsecondary

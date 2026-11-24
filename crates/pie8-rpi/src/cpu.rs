use core::arch::asm;

#[must_use]
pub fn current_el() -> u64 {
    let value: u64;
    // SAFETY: Reading CurrentEL has no side effects and is valid at every EL.
    unsafe { asm!("mrs {value}, CurrentEL", value = out(reg) value, options(nomem, nostack)) };
    value
}

#[must_use]
pub fn mpidr() -> u64 {
    let value: u64;
    // SAFETY: MPIDR_EL1 is readable at EL1 and above.
    unsafe { asm!("mrs {value}, MPIDR_EL1", value = out(reg) value, options(nomem, nostack)) };
    value
}

pub fn wait_forever() -> ! {
    loop {
        // SAFETY: `wfe` only puts this core into a low-power wait state.
        unsafe { asm!("wfe", options(nomem, nostack)) };
    }
}

#[inline(always)]
pub(crate) fn data_memory_barrier() {
    // SAFETY: The barrier has no memory operand; it orders explicit MMIO accesses.
    unsafe { asm!("dmb osh", options(nostack, preserves_flags)) };
}

#[inline(always)]
pub fn data_sync_barrier() {
    // SAFETY: A system-wide completion barrier is valid at EL1.
    unsafe { asm!("dsb sy", options(nostack, preserves_flags)) };
}

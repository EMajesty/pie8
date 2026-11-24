#![no_std]

/// Text printed after the architecture-specific runtime has initialized.
pub const RUST_READY_MESSAGE: &str = "Rust initialization: OK";

/// Converts an AArch64 `CurrentEL` register value to its exception-level number.
#[must_use]
pub const fn exception_level(current_el: u64) -> u8 {
    ((current_el >> 2) & 0b11) as u8
}

/// Returns the lowest affinity field from `MPIDR_EL1` (the local CPU number).
#[must_use]
pub const fn core_id(mpidr: u64) -> u8 {
    (mpidr & 0xff) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_exception_levels() {
        assert_eq!(exception_level(0b0100), 1);
        assert_eq!(exception_level(0b1000), 2);
        assert_eq!(exception_level(0b1100), 3);
    }

    #[test]
    fn decodes_core_affinity() {
        assert_eq!(core_id(0x8000_0002), 2);
    }
}

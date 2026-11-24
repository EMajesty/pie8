use crate::{PERIPHERAL_BASE, read32, write32};

const GPFSEL1: usize = PERIPHERAL_BASE + 0x20_0004;
const GPPUD: usize = PERIPHERAL_BASE + 0x20_0094;
const GPPUDCLK0: usize = PERIPHERAL_BASE + 0x20_0098;

/// Routes GPIO14/15 to ALT5 (mini UART) and disables their pulls.
pub(crate) fn configure_uart_pins() {
    let mut select = read32(GPFSEL1);
    select &= !((0b111 << 12) | (0b111 << 15));
    select |= (0b010 << 12) | (0b010 << 15);
    write32(GPFSEL1, select);

    // BCM2837 uses the legacy GPPUD sequence: write the control, wait 150 cycles,
    // clock the target pins, wait again, then remove control and clock.
    write32(GPPUD, 0);
    delay_cycles(150);
    write32(GPPUDCLK0, (1 << 14) | (1 << 15));
    delay_cycles(150);
    write32(GPPUDCLK0, 0);
}

#[inline(never)]
fn delay_cycles(count: usize) {
    for _ in 0..count {
        core::hint::spin_loop();
    }
}

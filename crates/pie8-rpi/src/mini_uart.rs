use core::fmt;

use crate::{PERIPHERAL_BASE, gpio, read32, timer::Timer, write32};

const AUX_BASE: usize = PERIPHERAL_BASE + 0x21_5000;
const AUX_ENABLES: usize = AUX_BASE + 0x04;
const AUX_MU_IO: usize = AUX_BASE + 0x40;
const AUX_MU_IER: usize = AUX_BASE + 0x44;
const AUX_MU_IIR: usize = AUX_BASE + 0x48;
const AUX_MU_LCR: usize = AUX_BASE + 0x4c;
const AUX_MU_MCR: usize = AUX_BASE + 0x50;
const AUX_MU_LSR: usize = AUX_BASE + 0x54;
const AUX_MU_CNTL: usize = AUX_BASE + 0x60;
const AUX_MU_BAUD: usize = AUX_BASE + 0x68;

/// Polling mini UART configured for 115200 8N1 from a fixed 250 MHz core clock.
pub struct MiniUart;

impl MiniUart {
    /// Initializes UART1. `config.txt` must contain `enable_uart=1` and
    /// `core_freq=250`, which make divisor 270 yield approximately 115200 baud.
    #[must_use]
    pub fn new() -> Self {
        let timer = Timer::new();
        write32(AUX_ENABLES, read32(AUX_ENABLES) | 1);
        write32(AUX_MU_CNTL, 0);
        write32(AUX_MU_IER, 0);
        write32(AUX_MU_LCR, 3);
        write32(AUX_MU_MCR, 0);
        write32(AUX_MU_IIR, 0xc6);
        write32(AUX_MU_BAUD, 270);
        gpio::configure_uart_pins(&timer);
        write32(AUX_MU_CNTL, 3);
        Self
    }

    pub fn write_byte(&mut self, byte: u8) {
        while read32(AUX_MU_LSR) & 0x20 == 0 {
            core::hint::spin_loop();
        }
        write32(AUX_MU_IO, u32::from(byte));
    }
}

impl Default for MiniUart {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for MiniUart {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

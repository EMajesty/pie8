use core::arch::asm;

#[derive(Clone, Copy)]
pub struct Timer {
    frequency_hz: u64,
}

impl Timer {
    #[must_use]
    pub fn new() -> Self {
        let frequency_hz: u64;
        // SAFETY: CNTFRQ_EL0 is readable at EL1 after startup configured EL2 traps.
        unsafe {
            asm!("mrs {value}, CNTFRQ_EL0", value = out(reg) frequency_hz, options(nomem, nostack))
        };
        assert!(frequency_hz != 0);
        Self { frequency_hz }
    }

    #[must_use]
    pub fn ticks(&self) -> u64 {
        let ticks: u64;
        // SAFETY: CNTPCT_EL0 is readable at EL1 after CNTHCTL_EL2 setup.
        unsafe { asm!("mrs {value}, CNTPCT_EL0", value = out(reg) ticks, options(nomem, nostack)) };
        ticks
    }

    #[must_use]
    pub const fn frequency_hz(&self) -> u64 {
        self.frequency_hz
    }

    #[must_use]
    pub fn micros(&self) -> u64 {
        scale_ticks(self.ticks(), self.frequency_hz, 1_000_000)
    }

    #[must_use]
    pub fn millis(&self) -> u64 {
        scale_ticks(self.ticks(), self.frequency_hz, 1_000)
    }

    pub fn delay_us(&self, duration_us: u64) {
        let duration_ticks = duration_us
            .saturating_mul(self.frequency_hz)
            .saturating_add(999_999)
            / 1_000_000;
        let start = self.ticks();
        while self.ticks().wrapping_sub(start) < duration_ticks {
            core::hint::spin_loop();
        }
    }

    pub fn delay_ms(&self, duration_ms: u64) {
        self.delay_us(duration_ms.saturating_mul(1_000));
    }

    #[must_use]
    pub fn deadline_expired(&self, start_ticks: u64, timeout_us: u64) -> bool {
        let timeout_ticks = timeout_us.saturating_mul(self.frequency_hz) / 1_000_000;
        self.ticks().wrapping_sub(start_ticks) > timeout_ticks
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

fn scale_ticks(ticks: u64, frequency: u64, units: u64) -> u64 {
    (ticks / frequency) * units + (ticks % frequency) * units / frequency
}

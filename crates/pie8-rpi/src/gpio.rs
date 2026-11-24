use crate::{PERIPHERAL_BASE, read32, timer::Timer, write32};

const GPIO_BASE: usize = PERIPHERAL_BASE + 0x20_0000;
const GPFSEL0: usize = GPIO_BASE;
const GPSET0: usize = GPIO_BASE + 0x1c;
const GPCLR0: usize = GPIO_BASE + 0x28;
const GPLEV0: usize = GPIO_BASE + 0x34;
const GPPUD: usize = GPIO_BASE + 0x94;
const GPPUDCLK0: usize = GPIO_BASE + 0x98;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Function {
    Input = 0b000,
    Output = 0b001,
    Alt5 = 0b010,
    Alt4 = 0b011,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Pull {
    None = 0,
    Down = 1,
    Up = 2,
}

#[derive(Clone, Copy)]
pub struct Pin {
    number: u8,
}

impl Pin {
    #[must_use]
    pub const fn new(number: u8) -> Self {
        assert!(number <= 53);
        Self { number }
    }

    pub fn set_function(self, function: Function) {
        let register = GPFSEL0 + usize::from(self.number / 10) * 4;
        let shift = u32::from((self.number % 10) * 3);
        let value = (read32(register) & !(0b111 << shift)) | ((function as u32) << shift);
        write32(register, value);
    }

    /// Configures a BCM2837 legacy pull using the required timed clock sequence.
    pub fn set_pull(self, pull: Pull, timer: &Timer) {
        write32(GPPUD, pull as u32);
        timer.delay_us(2);
        let clock = GPPUDCLK0 + usize::from(self.number / 32) * 4;
        write32(clock, 1 << (self.number % 32));
        timer.delay_us(2);
        write32(GPPUD, 0);
        write32(clock, 0);
    }

    #[must_use]
    pub fn level(self) -> bool {
        let register = GPLEV0 + usize::from(self.number / 32) * 4;
        read32(register) & (1 << (self.number % 32)) != 0
    }

    pub fn set_high(self) {
        let register = GPSET0 + usize::from(self.number / 32) * 4;
        write32(register, 1 << (self.number % 32));
    }

    pub fn set_low(self) {
        let register = GPCLR0 + usize::from(self.number / 32) * 4;
        write32(register, 1 << (self.number % 32));
    }

    pub fn write(self, high: bool) {
        if high {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

/// Routes GPIO14/15 to ALT5 (mini UART) and disables their pulls.
pub(crate) fn configure_uart_pins(timer: &Timer) {
    for pin in [Pin::new(14), Pin::new(15)] {
        pin.set_function(Function::Alt5);
        pin.set_pull(Pull::None, timer);
    }
}

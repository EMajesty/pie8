use crate::{
    PERIPHERAL_BASE,
    gpio::{Function, Pin, Pull},
    read32,
    timer::Timer,
    write32,
};

const SPI0_BASE: usize = PERIPHERAL_BASE + 0x20_4000;
const CS: usize = SPI0_BASE;
const FIFO: usize = SPI0_BASE + 0x04;
const CLK: usize = SPI0_BASE + 0x08;

const CS_CLEAR_RX_TX: u32 = 0b11 << 4;
const CS_TA: u32 = 1 << 7;
const CS_DONE: u32 = 1 << 16;
const CS_RXD: u32 = 1 << 17;
const CS_TXD: u32 = 1 << 18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiError {
    Timeout,
}

/// BCM2837 SPI0 in mode 0, hardware CE0, polling operation.
pub struct Spi0 {
    divider: u16,
    timeout_us: u64,
}

impl Spi0 {
    pub fn new(timer: &Timer, divider: u16, timeout_us: u64) -> Self {
        // MISO/GPIO9 is intentionally not configured: the prototype is a
        // write-only display connection, though the controller RX FIFO is
        // still drained for every transmitted byte.
        for pin in [Pin::new(8), Pin::new(10), Pin::new(11)] {
            pin.set_function(Function::Alt0);
            pin.set_pull(Pull::None, timer);
        }
        let divider = divider.max(2) & !1;
        write32(CS, CS_CLEAR_RX_TX); // mode 0, CE0 active-low, TA off
        write32(CLK, u32::from(divider));
        Self {
            divider,
            timeout_us,
        }
    }

    #[must_use]
    pub const fn divider(&self) -> u16 {
        self.divider
    }

    /// Core clock divided by the configured even divider (31.25 gives 8 MHz).
    #[must_use]
    pub const fn estimated_hz(&self, core_clock_hz: u32) -> u32 {
        core_clock_hz / self.divider as u32
    }

    pub fn write(&mut self, timer: &Timer, bytes: &[u8]) -> Result<(), SpiError> {
        if bytes.is_empty() {
            return Ok(());
        }
        write32(CS, CS_CLEAR_RX_TX | CS_TA);
        let start = timer.ticks();
        let mut sent = 0;
        let mut received = 0;
        while received < bytes.len() {
            let status = read32(CS);
            while sent < bytes.len() && read32(CS) & CS_TXD != 0 {
                write32(FIFO, u32::from(bytes[sent]));
                sent += 1;
            }
            while received < sent && read32(CS) & CS_RXD != 0 {
                let _ = read32(FIFO);
                received += 1;
            }
            if status & CS_DONE != 0 && received == bytes.len() {
                break;
            }
            if timer.deadline_expired(start, self.timeout_us) {
                write32(CS, CS_CLEAR_RX_TX);
                return Err(SpiError::Timeout);
            }
        }
        while read32(CS) & CS_DONE == 0 {
            if timer.deadline_expired(start, self.timeout_us) {
                write32(CS, CS_CLEAR_RX_TX);
                return Err(SpiError::Timeout);
            }
        }
        write32(CS, CS_CLEAR_RX_TX);
        Ok(())
    }

    /// Streams native RGB565 words as the display-required big-endian bytes.
    pub fn write_pixels(&mut self, timer: &Timer, pixels: &[u16]) -> Result<(), SpiError> {
        // Small chunks bound stack use and keep the same FIFO-safe transfer path.
        let mut bytes = [0_u8; 128];
        for chunk in pixels.chunks(bytes.len() / 2) {
            for (output, pixel) in bytes.chunks_exact_mut(2).zip(chunk) {
                output.copy_from_slice(&pixel.to_be_bytes());
            }
            self.write(timer, &bytes[..chunk.len() * 2])?;
        }
        Ok(())
    }
}

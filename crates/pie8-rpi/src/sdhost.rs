//! Read-only programmed-I/O driver for the BCM2835 SDHOST controller.
//!
//! The controller is the external microSD path on the supported BCM2837 boards.
//! All mutable state is single-core owned; no write or erase SD command exists here.

use core::cell::UnsafeCell;

use pie8_core::{
    block::{BLOCK_SIZE, BlockError, ReadBlockDevice},
    sd::{
        Addressing, capacity_blocks, decode_ocr, read_argument, relative_card_address, sdhost_clock,
    },
};

use crate::{
    PERIPHERAL_BASE,
    gpio::{Function, Pin, Pull},
    read32,
    timer::Timer,
    write32,
};

const BASE: usize = PERIPHERAL_BASE + 0x20_2000;
const SDCMD: usize = BASE;
const SDARG: usize = BASE + 0x04;
const SDTOUT: usize = BASE + 0x08;
const SDCDIV: usize = BASE + 0x0c;
const SDRSP0: usize = BASE + 0x10;
const SDHSTS: usize = BASE + 0x20;
const SDVDD: usize = BASE + 0x30;
const SDEDM: usize = BASE + 0x34;
const SDHCFG: usize = BASE + 0x38;
const SDHBCT: usize = BASE + 0x3c;
const SDDATA: usize = BASE + 0x40;
const SDHBLC: usize = BASE + 0x50;

const CMD_NEW: u32 = 0x8000;
const CMD_FAIL: u32 = 0x4000;
const CMD_BUSY: u32 = 0x0800;
const CMD_NO_RESPONSE: u32 = 0x0400;
const CMD_LONG: u32 = 0x0200;
const CMD_READ: u32 = 0x0040;
const HSTS_ERROR: u32 = 0x00f8;
const HSTS_BUSY: u32 = 0x0400;
const HCFG_BUSY_IRQ: u32 = 1 << 10;
const HCFG_DATA_IRQ: u32 = 1 << 4;
const HCFG_WIDE_EXT: u32 = 1 << 2;
const HCFG_WIDE_INT: u32 = 1 << 1;
const EDM_FIFO_COUNT_MASK: u32 = 0x1f0;
const EDM_FIFO_COUNT_SHIFT: u32 = 4;
const EDM_FORCE_DATA_MODE: u32 = 1 << 19;
const CORE_CLOCK_HZ: u32 = 250_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitStage {
    Reset,
    Idle,
    Interface,
    OperatingCondition,
    Identification,
    Addressed,
    Selected,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdError {
    Timeout,
    Command { command: u8, status: u32 },
    Response,
    UnsupportedCard,
    Capacity,
    OutOfRange,
}

#[derive(Clone, Copy, Debug)]
pub struct Diagnostics {
    pub stage: InitStage,
    pub last_command: u8,
    pub last_response: u32,
    pub status: u32,
    pub debug: u32,
    pub ocr: u32,
    pub rca: u16,
    pub cid: [u32; 4],
    pub csd: [u32; 4],
    pub blocks: u64,
    pub clock_hz: u32,
    pub last_error: Option<SdError>,
}

impl Diagnostics {
    const fn new() -> Self {
        Self {
            stage: InitStage::Reset,
            last_command: 0,
            last_response: 0,
            status: 0,
            debug: 0,
            ocr: 0,
            rca: 0,
            cid: [0; 4],
            csd: [0; 4],
            blocks: 0,
            clock_hz: 0,
            last_error: None,
        }
    }
}

struct State {
    diagnostics: Diagnostics,
    addressing: Addressing,
    initialized: bool,
}

pub struct SdHost {
    timer: Timer,
    state: UnsafeCell<State>,
}

// SAFETY: PIE8 parks secondary cores and the firmware never aliases an SdHost
// operation. UnsafeCell is required only because ReadBlockDevice uses `&self`.
unsafe impl Sync for SdHost {}

impl SdHost {
    #[must_use]
    pub const fn new(timer: Timer) -> Self {
        Self {
            timer,
            state: UnsafeCell::new(State {
                diagnostics: Diagnostics::new(),
                addressing: Addressing::Block,
                initialized: false,
            }),
        }
    }

    #[must_use]
    pub fn diagnostics(&self) -> Diagnostics {
        // SAFETY: Copy-only observation on the single active core.
        unsafe { (*self.state.get()).diagnostics }
    }

    pub fn initialize(&self) -> Result<(), SdError> {
        // SAFETY: Firmware calls this and block reads serially on core zero.
        let state = unsafe { &mut *self.state.get() };
        *state = State {
            diagnostics: Diagnostics::new(),
            addressing: Addressing::Block,
            initialized: false,
        };
        let result = self.initialize_inner(state);
        if let Err(error) = result {
            state.diagnostics.last_error = Some(error);
        }
        result
    }

    fn initialize_inner(&self, state: &mut State) -> Result<(), SdError> {
        self.configure_pins();
        write32(SDVDD, 0);
        write32(SDCMD, 0);
        write32(SDARG, 0);
        write32(SDTOUT, 0x00f0_0000);
        write32(SDCDIV, 0);
        write32(SDHSTS, 0x07f8);
        write32(SDHCFG, HCFG_BUSY_IRQ | HCFG_DATA_IRQ);
        write32(SDHBCT, 0);
        write32(SDHBLC, 0);
        let edm = read32(SDEDM);
        write32(SDEDM, (edm & !0x0007_ffff) | (4 << 14) | (4 << 9));
        self.timer.delay_ms(20);
        write32(SDVDD, 1);
        self.timer.delay_ms(20);
        self.set_clock(state, 400_000);

        self.command(state, 0, 0, Response::None, false, false)?;
        state.diagnostics.stage = InitStage::Idle;
        let interface = self.command(state, 8, 0x1aa, Response::Short, false, false)?[0];
        if interface & 0xfff != 0x1aa {
            return Err(SdError::UnsupportedCard);
        }
        state.diagnostics.stage = InitStage::Interface;

        let start = self.timer.ticks();
        let ocr = loop {
            self.command(state, 55, 0, Response::Short, false, false)?;
            let raw = self.command(state, 41, 0x40ff_8000, Response::Short, false, true)?[0];
            if decode_ocr(raw).ready {
                break raw;
            }
            if self.timer.deadline_expired(start, 1_000_000) {
                return Err(SdError::Timeout);
            }
            self.timer.delay_ms(1);
        };
        state.diagnostics.ocr = ocr;
        state.addressing = decode_ocr(ocr).addressing;
        state.diagnostics.stage = InitStage::OperatingCondition;

        state.diagnostics.cid = self.command(state, 2, 0, Response::Long, false, false)?;
        state.diagnostics.stage = InitStage::Identification;
        let r6 = self.command(state, 3, 0, Response::Short, false, false)?[0];
        state.diagnostics.rca = relative_card_address(r6).ok_or(SdError::Response)?;
        state.diagnostics.stage = InitStage::Addressed;
        state.diagnostics.csd = self.command(
            state,
            9,
            u32::from(state.diagnostics.rca) << 16,
            Response::Long,
            false,
            false,
        )?;
        state.diagnostics.blocks =
            capacity_blocks(state.diagnostics.csd).ok_or(SdError::Capacity)?;
        self.command(
            state,
            7,
            u32::from(state.diagnostics.rca) << 16,
            Response::Short,
            true,
            false,
        )?;
        state.diagnostics.stage = InitStage::Selected;

        self.command(
            state,
            55,
            u32::from(state.diagnostics.rca) << 16,
            Response::Short,
            false,
            false,
        )?;
        self.command(state, 6, 2, Response::Short, false, false)?;
        write32(
            SDHCFG,
            HCFG_BUSY_IRQ | HCFG_DATA_IRQ | HCFG_WIDE_EXT | HCFG_WIDE_INT,
        );
        if state.addressing == Addressing::Byte {
            self.command(state, 16, BLOCK_SIZE as u32, Response::Short, false, false)?;
        }
        self.set_clock(state, 25_000_000);
        state.diagnostics.stage = InitStage::Ready;
        state.initialized = true;
        Ok(())
    }

    fn configure_pins(&self) {
        Pin::new(48).set_function(Function::Alt0);
        Pin::new(48).set_pull(Pull::None, &self.timer);
        for number in 49..=53 {
            let pin = Pin::new(number);
            pin.set_function(Function::Alt0);
            pin.set_pull(Pull::Up, &self.timer);
        }
    }

    fn set_clock(&self, state: &mut State, requested: u32) {
        let (register, actual) = sdhost_clock(CORE_CLOCK_HZ, requested).unwrap();
        write32(SDCDIV, register);
        state.diagnostics.clock_hz = actual;
        let timeout = (state.diagnostics.clock_hz / 2).max(1);
        write32(SDTOUT, timeout);
    }

    fn command(
        &self,
        state: &mut State,
        command: u8,
        argument: u32,
        response: Response,
        busy: bool,
        ignore_crc7: bool,
    ) -> Result<[u32; 4], SdError> {
        let start = self.timer.ticks();
        while read32(SDCMD) & CMD_NEW != 0 {
            if self.timer.deadline_expired(start, 500_000) {
                return Err(SdError::Timeout);
            }
        }
        write32(SDHSTS, 0x07f8);
        write32(SDARG, argument);
        let mut value = u32::from(command) | CMD_NEW;
        if command == 17 {
            value |= CMD_READ;
        }
        value |= match response {
            Response::None => CMD_NO_RESPONSE,
            Response::Short => 0,
            Response::Long => CMD_LONG,
        };
        if busy {
            value |= CMD_BUSY;
        }
        state.diagnostics.last_command = command;
        write32(SDCMD, value);
        let start = self.timer.ticks();
        loop {
            let command_status = read32(SDCMD);
            let host_status = read32(SDHSTS);
            state.diagnostics.status = host_status;
            state.diagnostics.debug = read32(SDEDM);
            if command_status & CMD_NEW == 0 {
                if command_status & CMD_FAIL != 0 {
                    let remaining = if ignore_crc7 {
                        host_status & !0x10
                    } else {
                        host_status
                    };
                    if remaining & HSTS_ERROR != 0 {
                        return Err(SdError::Command {
                            command,
                            status: host_status,
                        });
                    }
                }
                break;
            }
            if self.timer.deadline_expired(start, 500_000) {
                return Err(SdError::Timeout);
            }
        }
        if busy {
            let start = self.timer.ticks();
            while read32(SDHSTS) & HSTS_BUSY == 0 {
                if self.timer.deadline_expired(start, 500_000) {
                    return Err(SdError::Timeout);
                }
            }
            write32(SDHSTS, HSTS_BUSY);
        }
        let words = match response {
            Response::Long => [
                read32(SDRSP0),
                read32(SDRSP0 + 4),
                read32(SDRSP0 + 8),
                read32(SDRSP0 + 12),
            ],
            _ => [read32(SDRSP0), 0, 0, 0],
        };
        state.diagnostics.last_response = words[0];
        Ok(words)
    }

    fn read_block_inner(
        &self,
        state: &mut State,
        block: u64,
        out: &mut [u8],
    ) -> Result<(), SdError> {
        if !state.initialized || block >= state.diagnostics.blocks {
            return Err(SdError::OutOfRange);
        }
        let argument = read_argument(block, state.addressing).ok_or(SdError::OutOfRange)?;
        write32(SDHBCT, BLOCK_SIZE as u32);
        write32(SDHBLC, 1);
        self.command(state, 17, argument, Response::Short, false, false)?;
        let start = self.timer.ticks();
        let mut offset = 0;
        while offset < BLOCK_SIZE {
            let status = read32(SDHSTS);
            state.diagnostics.status = status;
            let mut debug = read32(SDEDM);
            state.diagnostics.debug = debug;
            if status & HSTS_ERROR != 0 {
                return Err(SdError::Command {
                    command: 17,
                    status,
                });
            }
            let count = (debug & EDM_FIFO_COUNT_MASK) >> EDM_FIFO_COUNT_SHIFT;
            if count != 0 {
                let word = read32(SDDATA).to_le_bytes();
                out[offset..offset + 4].copy_from_slice(&word);
                offset += 4;
                continue;
            }
            if debug & 0xf == 5 {
                debug |= EDM_FORCE_DATA_MODE;
                write32(SDEDM, debug);
            }
            if self.timer.deadline_expired(start, 500_000) {
                return Err(SdError::Timeout);
            }
        }
        write32(SDHSTS, read32(SDHSTS) & 0x07f8);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Response {
    None,
    Short,
    Long,
}

impl ReadBlockDevice for SdHost {
    fn block_count(&self) -> u64 {
        self.diagnostics().blocks
    }

    fn read_blocks(&self, first: u64, destination: &mut [u8]) -> Result<(), BlockError> {
        if destination.is_empty() || destination.len() % BLOCK_SIZE != 0 {
            return Err(BlockError::InvalidBuffer);
        }
        let count = destination.len() / BLOCK_SIZE;
        let end = first
            .checked_add(count as u64)
            .ok_or(BlockError::OutOfRange)?;
        if end > self.block_count() {
            return Err(BlockError::OutOfRange);
        }
        // SAFETY: See the type-level single-core invariant above.
        let state = unsafe { &mut *self.state.get() };
        for (index, block) in destination.chunks_exact_mut(BLOCK_SIZE).enumerate() {
            if let Err(error) = self.read_block_inner(state, first + index as u64, block) {
                state.diagnostics.last_error = Some(error);
                return Err(match error {
                    SdError::Timeout => BlockError::Timeout,
                    SdError::OutOfRange => BlockError::OutOfRange,
                    SdError::UnsupportedCard => BlockError::Unsupported,
                    _ => BlockError::Io,
                });
            }
        }
        Ok(())
    }
}

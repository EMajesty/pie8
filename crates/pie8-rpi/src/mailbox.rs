//! VideoCore property mailbox access for BCM2837-class boards.
use crate::{PERIPHERAL_BASE, read32, timer::Timer, write32};
use core::cell::UnsafeCell;
const BASE: usize = PERIPHERAL_BASE + 0x00b880;
const READ: usize = BASE;
const STATUS: usize = BASE + 0x18;
const WRITE: usize = BASE + 0x20;
const EMPTY: u32 = 1 << 30;
const FULL: u32 = 1 << 31;
const PROPERTY: u32 = 8;
const GET_ARM_MEMORY: u32 = 0x0001_0005;
const GET_BOARD_REVISION: u32 = 0x0001_0002;
#[repr(C, align(16))]
struct Message(UnsafeCell<[u32; 8]>);
unsafe impl Sync for Message {}
static MESSAGE: Message = Message(UnsafeCell::new([0; 8]));
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArmMemory {
    pub base: u32,
    pub size: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxError {
    Timeout,
    Address,
    Response,
    Tag,
    Range,
}

/// Queries the firmware-owned ARM memory split before the MMU is enabled.
pub fn arm_memory(timer: &Timer, timeout_us: u64) -> Result<ArmMemory, MailboxError> {
    let address = MESSAGE.0.get() as usize;
    if address & 15 != 0 || address > 0x3fff_fff0 {
        return Err(MailboxError::Address);
    }
    // Property mailbox messages use the 16-byte-aligned ARM physical address.
    let request_address = address as u32;
    let request = (request_address & !15) | PROPERTY;
    unsafe { *MESSAGE.0.get() = [32, 0, GET_ARM_MEMORY, 8, 0, 0, 0, 0] }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    let start = timer.ticks();
    while read32(STATUS) & FULL != 0 {
        if timer.deadline_expired(start, timeout_us) {
            return Err(MailboxError::Timeout);
        }
    }
    write32(WRITE, request);
    loop {
        while read32(STATUS) & EMPTY != 0 {
            if timer.deadline_expired(start, timeout_us) {
                return Err(MailboxError::Timeout);
            }
        }
        let response = read32(READ);
        if response & 15 != PROPERTY {
            continue;
        }
        if response & !15 != request_address & !15 {
            return Err(MailboxError::Response);
        }
        break;
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
    let words = unsafe { &*MESSAGE.0.get() };
    if words[0] != 32 || words[1] != 0x8000_0000 {
        return Err(MailboxError::Response);
    }
    if words[2] != GET_ARM_MEMORY
        || words[3] != 8
        || words[4] & 0x8000_0000 == 0
        || words[4] & 0x7fff_ffff < 8
    {
        return Err(MailboxError::Tag);
    }
    let memory = ArmMemory {
        base: words[5],
        size: words[6],
    };
    if memory.base != 0 || memory.size < 16 * 1024 * 1024 || memory.size > 0x4000_0000 {
        return Err(MailboxError::Range);
    }
    Ok(memory)
}

/// Queries the firmware board-revision code. This is diagnostic information;
/// SD routing is deliberately fixed rather than inferred from this value.
pub fn board_revision(timer: &Timer, timeout_us: u64) -> Result<u32, MailboxError> {
    let address = MESSAGE.0.get() as usize;
    if address & 15 != 0 || address > 0x3fff_fff0 {
        return Err(MailboxError::Address);
    }
    let request_address = address as u32;
    let request = request_address | PROPERTY;
    unsafe { *MESSAGE.0.get() = [32, 0, GET_BOARD_REVISION, 4, 0, 0, 0, 0] }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    let start = timer.ticks();
    while read32(STATUS) & FULL != 0 {
        if timer.deadline_expired(start, timeout_us) {
            return Err(MailboxError::Timeout);
        }
    }
    write32(WRITE, request);
    loop {
        while read32(STATUS) & EMPTY != 0 {
            if timer.deadline_expired(start, timeout_us) {
                return Err(MailboxError::Timeout);
            }
        }
        let response = read32(READ);
        if response & 15 != PROPERTY {
            continue;
        }
        if response & !15 != request_address {
            return Err(MailboxError::Response);
        }
        break;
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
    let words = unsafe { &*MESSAGE.0.get() };
    if words[0] != 32
        || words[1] != 0x8000_0000
        || words[2] != GET_BOARD_REVISION
        || words[3] != 4
        || words[4] & 0x8000_0000 == 0
        || words[4] & 0x7fff_ffff < 4
    {
        return Err(MailboxError::Tag);
    }
    Ok(words[5])
}

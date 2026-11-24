#![no_std]

pub mod cpu;
pub mod gpio;
pub mod mini_uart;

const PERIPHERAL_BASE: usize = 0x3f00_0000;

#[inline(always)]
pub(crate) fn read32(address: usize) -> u32 {
    // SAFETY: Callers supply an aligned address for a documented MMIO register.
    unsafe { core::ptr::read_volatile(address as *const u32) }
}

#[inline(always)]
pub(crate) fn write32(address: usize, value: u32) {
    // SAFETY: Callers supply an aligned address for a documented MMIO register.
    unsafe { core::ptr::write_volatile(address as *mut u32, value) }
}

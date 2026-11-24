#![no_std]

pub mod cpu;
pub mod gpio;
pub mod memory;
pub mod mini_uart;
pub mod spi;
pub mod timer;

const PERIPHERAL_BASE: usize = 0x3f00_0000;

core::arch::global_asm!(include_str!("cache.S"));

#[inline(always)]
pub(crate) fn read32(address: usize) -> u32 {
    cpu::data_memory_barrier();
    // SAFETY: Callers supply an aligned address for a documented MMIO register.
    let value = unsafe { core::ptr::read_volatile(address as *const u32) };
    cpu::data_memory_barrier();
    value
}

#[inline(always)]
pub(crate) fn write32(address: usize, value: u32) {
    cpu::data_memory_barrier();
    // SAFETY: Callers supply an aligned address for a documented MMIO register.
    unsafe { core::ptr::write_volatile(address as *mut u32, value) };
    cpu::data_memory_barrier();
}

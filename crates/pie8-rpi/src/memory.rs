use core::{arch::asm, cell::UnsafeCell};

const TABLE_ENTRIES: usize = 512;
const BLOCK_SIZE: u64 = 2 * 1024 * 1024;
pub const FALLBACK_RAM_BYTES: u64 = 16 * 1024 * 1024;
const PERIPHERAL_START: u64 = 0x3f00_0000;
const PERIPHERAL_END: u64 = 0x4000_0000;

const DESCRIPTOR_VALID_BLOCK: u64 = 0b01;
const DESCRIPTOR_VALID_TABLE: u64 = 0b11;
const ACCESS_FLAG: u64 = 1 << 10;
const INNER_SHAREABLE: u64 = 0b11 << 8;
const ATTR_DEVICE: u64 = 1 << 2;
const PXN: u64 = 1 << 53;
const UXN: u64 = 1 << 54;

#[repr(C, align(4096))]
struct Table(UnsafeCell<[u64; TABLE_ENTRIES]>);

// SAFETY: Only boot core 0 mutates these tables, once, before the MMU is enabled.
unsafe impl Sync for Table {}

#[unsafe(link_section = ".translation_tables")]
static L1_TABLE: Table = Table(UnsafeCell::new([0; TABLE_ENTRIES]));
#[unsafe(link_section = ".translation_tables")]
static L2_TABLE: Table = Table(UnsafeCell::new([0; TABLE_ENTRIES]));

unsafe extern "C" {
    fn invalidate_data_caches();
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryMap {
    pub detected_ram_bytes: Option<u64>,
    pub ram_mapped_bytes: u64,
    pub peripheral_start: u64,
    pub peripheral_end: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct OwnedRegion {
    pub start: usize,
    pub length: usize,
}

unsafe extern "C" {
    static __image_end: u8;
}

/// Returns the cacheable region exclusively available to the sample arena.
/// The linked image owns everything below `__image_end`; the firmware-reported
/// mapped limit bounds the other end. Future DMA pools must be carved out first.
pub fn sample_region(map: MemoryMap) -> Option<OwnedRegion> {
    let image_end = core::ptr::addr_of!(__image_end) as usize;
    let start = image_end.checked_add(63)? & !63;
    let end = usize::try_from(map.ram_mapped_bytes).ok()?;
    (start < end).then_some(OwnedRegion {
        start,
        length: end - start,
    })
}

/// Builds an identity map and enables EL1 stage-1 translation and both caches.
///
/// Only the first 16 MiB is mapped as usable Normal memory. This contains the
/// complete firmware image, tables, framebuffer, and stack on both supported
/// boards without claiming the firmware-reserved top of physical RAM.
pub fn enable_mmu_and_caches(detected_ram_bytes: Option<u64>) -> MemoryMap {
    let verified = detected_ram_bytes
        .filter(|size| *size >= FALLBACK_RAM_BYTES && *size <= PERIPHERAL_START)
        .unwrap_or(FALLBACK_RAM_BYTES);
    // Never map a partial 2 MiB block beyond firmware-reported ARM memory.
    let ram_mapped_bytes = verified / BLOCK_SIZE * BLOCK_SIZE;
    // SAFETY: Core 0 is the only running core; translation is disabled and these
    // aligned tables are exclusively owned until this initialization completes.
    unsafe {
        let l1 = &mut *L1_TABLE.0.get();
        let l2 = &mut *L2_TABLE.0.get();
        l1.fill(0);
        l2.fill(0);

        l1[0] = (L2_TABLE.0.get() as u64 & 0x0000_ffff_ffff_f000) | DESCRIPTOR_VALID_TABLE;
        for address in (0..ram_mapped_bytes).step_by(BLOCK_SIZE as usize) {
            l2[(address / BLOCK_SIZE) as usize] =
                address | DESCRIPTOR_VALID_BLOCK | ACCESS_FLAG | INNER_SHAREABLE;
        }
        for address in (PERIPHERAL_START..PERIPHERAL_END).step_by(BLOCK_SIZE as usize) {
            l2[(address / BLOCK_SIZE) as usize] = address
                | DESCRIPTOR_VALID_BLOCK
                | ACCESS_FLAG
                | INNER_SHAREABLE
                | ATTR_DEVICE
                | PXN
                | UXN;
        }

        let mair = 0x00ff_u64; // Attr0 Normal WBWA; Attr1 Device-nGnRnE.
        let tcr = 32_u64       // T0SZ: 4 GiB lower VA range.
            | (1 << 8)        // IRGN0: inner write-back, read/write allocate.
            | (1 << 10)       // ORGN0: outer write-back, read/write allocate.
            | (0b11 << 12)    // SH0: inner shareable.
            | (1 << 23); // EPD1: disable TTBR1 walks.
        let ttbr0 = L1_TABLE.0.get() as u64;
        asm!(
            "dsb sy",
            "msr MAIR_EL1, {mair}",
            "msr TCR_EL1, {tcr}",
            "msr TTBR0_EL1, {ttbr0}",
            "isb",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            mair = in(reg) mair, tcr = in(reg) tcr, ttbr0 = in(reg) ttbr0,
            options(nostack)
        );
        invalidate_data_caches();
        asm!("ic iallu", "dsb sy", "isb", options(nostack));
        let mut sctlr: u64;
        asm!("mrs {value}, SCTLR_EL1", value = out(reg) sctlr, options(nomem, nostack));
        sctlr |= (1 << 0) | (1 << 2) | (1 << 12); // M, C, I.
        asm!("msr SCTLR_EL1, {value}", "isb", value = in(reg) sctlr, options(nostack));
    }
    MemoryMap {
        detected_ram_bytes,
        ram_mapped_bytes,
        peripheral_start: PERIPHERAL_START,
        peripheral_end: PERIPHERAL_END,
    }
}

/// Makes CPU writes visible before a DMA engine reads a cacheable buffer.
pub fn clean_for_dma(address: usize, length: usize) {
    cache_range(address, length, "cvac");
}

/// Discards CPU cache lines after a DMA engine has written a buffer.
/// The caller must ensure the CPU has made no dirty writes while DMA owned it.
pub fn invalidate_after_dma(address: usize, length: usize) {
    cache_range(address, length, "ivac");
}

fn cache_range(address: usize, length: usize, operation: &str) {
    let start = address & !63;
    let end = address.saturating_add(length).saturating_add(63) & !63;
    let mut line = start;
    while line < end {
        // SAFETY: Cache maintenance by virtual address does not dereference it.
        unsafe {
            match operation {
                "cvac" => asm!("dc cvac, {line}", line = in(reg) line, options(nostack)),
                "ivac" => asm!("dc ivac, {line}", line = in(reg) line, options(nostack)),
                _ => unreachable!(),
            }
        }
        line += 64;
    }
    // SAFETY: Complete cache maintenance before ownership changes.
    unsafe { asm!("dsb sy", options(nostack)) };
}

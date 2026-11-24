# Architecture and technical decisions

## EL1 memory system

Startup still normalizes to EL1h, enables FP/SIMD, installs aligned exception
vectors, initializes BSS/stack, and parks secondary cores. Rust first captures
an uncached CPU/memory benchmark, constructs translation tables, invalidates
data/instruction caches and TLBs, then enables `SCTLR_EL1.M/C/I`.

The 4 KiB-granule, 32-bit VA identity map uses one level-1 table and one
level-2 table with 2 MiB blocks:

- `0x00000000..0x01000000`: Normal, inner-shareable, inner/outer write-back,
  read/write-allocate memory. Only 16 MiB is declared usable, enough for the
  image, benchmark scratch, tables, framebuffer, and stack on both targets.
- `0x3f000000..0x40000000`: Device-nGnRnE, inner-shareable, execute-never MMIO.
- Every other address is unmapped and faults rather than claiming nonexistent
  or firmware/GPU-reserved RAM.

`MAIR_EL1` contains Normal WBWA and Device-nGnRnE attributes; `TCR_EL1` selects
4 KiB TTBR0 walks, inner-shareable WBWA table walks, a 4 GiB lower address
range, and disables TTBR1. Existing UART fault handling remains identity-mapped
after MMU enablement, so synchronous translation faults can still report ESR,
ELR, FAR, and SPSR before parking.

MMIO remains volatile and bracketed by `dmb osh`, even though the page-table
Device type already prevents speculative/reordered device access. Translation
tables are two 4 KiB-aligned `NOLOAD` pages. Linker and host checks keep all
runtime sections below the 16 MiB map.

## DMA coherency contract

There is no active DMA buffer in this milestone. Future DMA buffers must be at
least 64-byte cache-line aligned, have exclusive CPU-or-device ownership, and
use the physical-to-bus mapping verified for the selected board/firmware.
Before DMA reads a cacheable buffer, firmware calls `clean_for_dma`; after DMA
writes and relinquishes ownership, firmware calls `invalidate_after_dma` before
CPU access. The CPU must not write a device-owned line. Control blocks require
the peripheral's documented alignment and the same ownership rules. A `dsb sy`
completes each ownership transition. These primitives exist now, but no backend
is allowed to use an assumed DMA bus alias.

## Performance diagnostics

Reproducible loops measure integer operations, floating-point operations, and
16 KiB memory copies before and after caches. Cached rendering measures 200
clipped RGB565 rectangles. Rates use generic-counter microseconds and portable,
host-tested calculation. `black_box` prevents removal of benchmark work.

SPI tests use actual achievable divisors of the fixed 250 MHz core clock:

| Divider | Calculated SPI clock |
|---:|---:|
| 32 | 7.8125 MHz |
| 16 | 15.625 MHz |
| 10 | 25 MHz |
| 8 | 31.25 MHz |

Each speed measures a complete 320×240 RGB565 transfer and a 64×64 region.
The benchmark always restores divider 32. The colour page's B4 also restores it
without reflashing. Values are on-screen and remain unverified until captured
on the Pi 3. Display transfers are blocking; no physical audio is active, and a
future DMA audio backend must remain independent of these stalls.

## Display correction

The reported cyan/magenta/yellow/black sequence is consistent with the panel
applying inversion to red/green/blue/white. The old default sent `INVON`; the new
default explicitly sends `INVOFF`. Runtime commands can toggle inversion and
the `MADCTL` RGB/BGR bit independently. The eight labelled swatches distinguish
inversion from red/blue order. MISO remains disconnected, so ST7789 identity is
not asserted.

## Portable audio

`pie8-core::audio` defines a peripheral-independent, allocation-free processor
over caller-owned blocks of stereo `f32` frames at nominal 48 kHz. Block size is
the slice length. `SignalGenerator` supports silence, corrected-parabolic sine,
triangle, and square waves; frequency is limited below Nyquist, gain is clamped
to 0..1, and final samples are clipped to -1..1. Phase continues across blocks.
Silence with zero gain is the boot default.

The audio page schedules 128-frame generator blocks and reports generated/late
blocks. This validates processing cost and controls only; it is not playback,
and “underrun” is N/A without a peripheral consumer.

## PWM analog-audio investigation and blocker

The BCM peripheral specification describes PWM FIFO/DREQ and DMA engines, but
not the complete clock-manager interface required to select and stabilize the
PWM source/divider. The Pi 3 jack is commonly described as PWM channels routed
internally through GPIO40 and GPIO45, which are not header pins; authoritative
board-revision routing and startup state have not yet been established here.
DMA additionally requires a verified free channel and correct SDRAM bus alias,
which can vary with firmware/coherency configuration.

Consequently no speculative PWM, clock-manager, or DMA register writes are
present. A reliable backend requires: confirmed internal routing on the tested
Pi 3 revision; measured PWM clock/carrier and achievable 48 kHz scheme;
firmware/mailbox or otherwise authoritative DMA channel and bus-address policy;
aligned double buffers; clean start/stop; and hardware underrun evidence. The
future PCM5102A I²S backend will be a separate implementation of the same audio
processing interface.

## References

- [Arm memory-management guide](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/LearnTheArchitecture-MemoryManagement-101811_0100_00_en.pdf)
- [Armv8-A memory-model guide](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/Armv8-A%20memory%20model%20guide.pdf)
- [BCM2835 ARM Peripherals](https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf)
- [Arm generic timer guide](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/Generic%20Timer.pdf)
- [ST7789V specification](https://dl.espressif.com/dl/schematics/ST7789V_SPEC_V1.0.pdf)

No third-party driver implementation was copied.

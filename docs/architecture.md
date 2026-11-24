# Architecture and technical decisions

## Boot memory discovery and MMU

Before enabling translation, core 0 sends the firmware property tag **Get ARM
memory** (`0x00010005`) over mailbox channel 8. The 16-byte-aligned request uses
the aligned ARM physical buffer address, a 100 ms generic-counter timeout, and validates
the returned buffer address, global response, tag ID, response bit/length,
zero base, and a 16 MiB–1 GiB size. Failure selects a conservative 16 MiB map.

The 4 KiB translation tables identity-map only complete 2 MiB blocks within the
verified ARM-accessible size as Normal WBWA inner-shareable memory. The
`0x3f000000..0x40000000` peripheral window remains Device-nGnRnE and XN;
everything else faults. Firmware/GPU-reserved top memory is excluded because
the property returns the ARM allocation rather than installed capacity. This
works for the BCM2837 Pi 3 and BCM2710/BCM2837-derived Zero 2 W address layout;
other SoCs are out of scope.

The linker image owns `[0x80000, __image_end)`. A 64-byte-aligned sample region
owns `[align_up(__image_end), mapped_ram_end)`. Allocation is monotonic,
alignment-aware, checked for overflow/exhaustion, and reset as a unit. Future
DMA pools must be carved from this region before constructing the sample arena.
Audio processing never allocates. DMA cache clean/invalidate routines retain
their exclusive CPU/device ownership contract and 64-byte cache-line rules.

## Read-only storage layers

`ReadBlockDevice` exposes only block count and reads of complete 512-byte
blocks. Implementations are an in-memory fixture and a seekable host image.
There is intentionally no write method. The Raspberry Pi physical implementation
is deferred because runtime routing between SDHOST and Arasan is unresolved.

The internal FAT32 reader was chosen over `embedded-sdmmc` 0.9 after evaluation.
That library is mature, no_std, allocation-free, and includes LFN iteration, but
its public API also contains filesystem mutation. PIE8's smaller reader makes
the read-only security boundary explicit and avoids pulling unused SPI-card and
write machinery. It validates arithmetic and device bounds, MBR type, BPB,
cluster numbers, bad/reserved/end markers, and caps traversal at cluster count.

Supported: MBR FAT32 (`0x0b`, `0x0c`), 512-byte sectors, nested directories,
8.3 and ASCII LFN names, fragmented files, metadata, and sequential file reads.
Unsupported: superfloppy, GPT, extended partitions, FAT12/16, exFAT, Unicode
LFN decoding, timestamps beyond raw directory metadata, writes, and hot-plug.

## WAV and sample representation

The RIFF parser accepts PCM format 1, mono/stereo, 16/24-bit little-endian, and
exactly 48 kHz. It walks chunks in any order, skips unknown chunks and odd-byte
padding, and rejects truncation, size overflow, missing chunks, and unsupported
rates instead of playing at the wrong pitch. PCM converts to stereo `f32`
frames in `[-1, 1)` when read.

Samples retain packed native PCM in RAM: PCM16 stereo uses 4 bytes/frame and
PCM24 stereo 6, versus 8 for float stereo. Metadata records ID, bounded filename,
channels, rate, bit depth, frames, duration, arena bytes, and offset. Reset
unloads the entire initial library; individual compaction is intentionally absent.

## SD controller blocker

BCM2837 has Arasan SDHCI and Broadcom SDHOST. Established Pi 3 configurations
route SDHOST to the external microSD pins 48–53 and Arasan to SDIO Wi-Fi, but
firmware/device-tree configuration can swap them. Zero 2 W also has onboard
Wi-Fi. PIE8 does not yet parse the firmware device tree or query an equivalent
routing contract, and the published peripheral manual does not fully define
the board routing/clock setup. Implementing one controller unconditionally
could address Wi-Fi rather than the card. No physical init, capacity, throughput,
or recovery claim is therefore made.

## References

- [Raspberry Pi property mailbox](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
- [BCM2835 ARM peripherals](https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf)
- [Linux BCM2835 SDHOST driver and routing notes](https://github.com/torvalds/linux/blob/master/drivers/mmc/host/bcm2835.c)
- [Zero W device-tree SD routing](https://github.com/torvalds/linux/blob/master/arch/arm/boot/dts/broadcom/bcm2835-rpi-zero-w.dts)
- [embedded-sdmmc](https://github.com/rust-embedded-community/embedded-sdmmc-rs)
- [Arm memory-management guide](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/LearnTheArchitecture-MemoryManagement-101811_0100_00_en.pdf)

No third-party driver code was copied.

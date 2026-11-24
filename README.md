# PIE8

PIE8 is a portable bare-metal Rust music tracker targeting Raspberry Pi 3 for
development and Raspberry Pi Zero 2 W on a future custom carrier. Milestone 004
adds firmware-reported memory discovery, a sample arena, and portable read-only
MBR/FAT32/WAV tooling. There is no Linux runtime on the target.

## Verified baseline

Milestones 002 and 003 have booted successfully on a physical Pi 3. Display
initialization/text, all eight buttons, indicators/counting, diagnostic pages,
inversion OFF with RGB order, and every selectable SPI rate worked. This is a
functional observation, not signal-integrity or long-duration qualification at
31.25 MHz. Quantitative benchmark values and UART remain untested. Milestone 004
memory discovery and storage behavior are not physically verified.

Prototype defaults are inversion OFF, RGB order, and 31.25 MHz SPI. Colour-page
B3 cycles rates and B4 restores the conservative 7.8125 MHz fallback. B7/B8
navigate all Buttons, Colours, Performance, Audio, Storage, and Samples pages.

## Build and boot-card update

```sh
nix develop
./scripts/check.sh
./scripts/build.sh
./scripts/prepare-sd.sh /run/media/$USER/PIE8BOOT
sync
```

`build/kernel8.img` is the bootable image. `prepare-sd.sh` requires an empty,
already mounted FAT partition and never formats or mounts media. Raspberry Pi
firmware is checksum-pinned.

To install samples on an existing mounted boot partition without formatting it:

```sh
./scripts/install-samples.sh /run/media/$USER/PIE8BOOT ./my-samples
sync
```

The files are copied below `/samples`. The current target cannot read them yet;
this establishes the intended layout while the physical SD controller blocker
is resolved.

## Reproducible host storage tests

```sh
./scripts/generate-storage-fixtures.sh /tmp/pie8-fixtures
cargo run -p pie8-storage-inspect -- image /tmp/pie8-fixtures/pie8-fat32.img
cargo run -p pie8-storage-inspect -- image /tmp/pie8-fixtures/pie8-fat32.img samples/drums/kick.wav
cargo run -p pie8-storage-inspect -- wav /tmp/pie8-fixtures/samples/drums/kick.wav
```

The generator creates a partitioned FAT32 image plus 48 kHz stereo PCM16 and
PCM24 WAVs. It only creates/overwrites paths under the explicitly supplied
output directory; it never accepts a block device.

## Current storage scope

Portable code supports bounded MBR discovery, FAT32 validation, root/nested
directories, short and long filenames, fragmented cluster chains, sequential
reads, and a host disk-image block device. WAV supports mono/stereo PCM16/24 at
exactly 48 kHz, unknown/padded/noncanonical chunks, and float conversion. Native
packed PCM is retained in the sample arena to use 25–50% less RAM than stereo
float storage; DSP conversion happens when consumed.

Only MBR partition types `0x0b`/`0x0c`, 512-byte sectors, FAT32, and ASCII LFN
characters are supported. GPT, exFAT, FAT12/16, Unicode LFN conversion,
resampling, writes, and hot-plug are unsupported.

Physical SD access is blocked because Pi 3/Zero 2 W firmware can route the
microSD and Wi-Fi between Arasan SDHCI and Broadcom SDHOST, while PIE8 has no
device-tree/routing query yet. No speculative controller registers and no SD
write/erase commands were added. PWM audio is retired from active work; audio
development remains host-side until I²S hardware arrives.

See [architecture](docs/architecture.md), [hardware/test procedure](docs/hardware.md),
and [roadmap](docs/roadmap.md).

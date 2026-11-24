# PIE8

PIE8 is a portable bare-metal Rust music tracker targeting Raspberry Pi 3 for
development and Raspberry Pi Zero 2 W on a future custom carrier. Milestone
004B adds read-only physical boot-card access through BCM2835 SDHOST, connected
to the existing MBR/FAT32/WAV stack. There is no Linux runtime on the target.

## Verified baseline

Milestones 002 and 003 have booted successfully on a physical Pi 3. Display
initialization/text, all eight buttons, indicators/counting, diagnostic pages,
inversion OFF with RGB order, and every selectable SPI rate worked. This is a
functional observation, not signal-integrity or long-duration qualification at
31.25 MHz. Quantitative benchmark values and UART remain untested. Milestone 004
memory discovery and all 004B SD behavior are not physically verified.

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

The files are copied below `/samples`; no card write API exists in firmware.

To generate and install the complete diagnostic fixture set:

```sh
./scripts/generate-storage-fixtures.sh /tmp/pie8-fixtures
./scripts/install-samples.sh /run/media/$USER/PIE8BOOT /tmp/pie8-fixtures/samples/*
sync
```

This adds mono PCM16, stereo PCM16, stereo PCM24, a four-second sample, nested
directories, a long filename, and an intentionally unsupported 44.1 kHz WAV.
It does not format the card or remove unrelated contents.

## Reproducible host storage tests

```sh
./scripts/generate-storage-fixtures.sh /tmp/pie8-fixtures
cargo run -p pie8-storage-inspect -- image /tmp/pie8-fixtures/pie8-fat32.img
cargo run -p pie8-storage-inspect -- image /tmp/pie8-fixtures/pie8-fat32.img samples/drums/kick.wav
cargo run -p pie8-storage-inspect -- wav /tmp/pie8-fixtures/samples/drums/kick.wav
```

The generator creates a partitioned FAT32 image plus the diagnostic WAV set.
It only creates/overwrites paths under the explicitly supplied
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

Physical access now uses the established Pi 3/Zero 2 W external-card route:
BCM2835 SDHOST at `0x3f202000`, GPIO48--53 ALT0. The Arasan/GPIO34--39 Wi-Fi
path is not initialized. Reads use bounded polling and CMD17; a multi-block API
request is currently a sequence of single-block operations. See the detailed
[controller decision](docs/sd-controller.md). No SD write/erase command exists.
The Storage page exposes raw bring-up diagnostics and B1 retry. The Samples
page enumerates `/samples`, enters nested directories, loads supported WAVs into
the existing arena, displays metadata/waveform/load time, and releases the
arena with B2. A large polling read can temporarily delay button polling; this
is measured during hardware validation rather than hidden behind a scheduler.

For a card already staged by milestone 004, replacing `kernel8.img` alone is
sufficient: 004B retains the same `config.txt` and pinned firmware bundle. A
new card still requires `prepare-sd.sh`.

See [architecture](docs/architecture.md), [hardware/test procedure](docs/hardware.md),
and [roadmap](docs/roadmap.md).

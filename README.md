# PIE8

PIE8 is an experimental portable hardware music tracker, inspired by the
Dirtywave M8, written in bare-metal Rust. The development board is a Raspberry
Pi 3; the intended product board is a Raspberry Pi Zero 2 W. Milestone 001 is
deliberately small: start one Cortex-A53 core in AArch64 EL1 and report a known
boot state over UART. There is no display, audio, storage, USB, allocator, or
tracker implementation yet.

## Repository layout

- `crates/pie8-core`: platform-independent logic, usable by firmware and host tests.
- `crates/pie8-rpi`: BCM2837 GPIO, CPU, and mini-UART primitives.
- `firmware`: AArch64 reset code, memory layout, panic handler, and boot banner.
- `tools/image-inspect`: host-side raw-image sanity checker.
- `boot`: files maintained by PIE8 for the FAT boot partition.
- `scripts`: reproducible build, verification, and SD staging commands.

See [architecture](docs/architecture.md) and [roadmap](docs/roadmap.md).

## Development and build

On NixOS (or another system with flakes enabled):

```sh
nix develop
./scripts/check.sh
```

The shorter build-only command is:

```sh
nix develop --command ./scripts/build.sh
```

It cross-compiles `aarch64-unknown-none`, converts the ELF to a flat binary,
and produces `build/kernel8.img`. The banner's build ID is the current short
Git commit by default; set `PIE8_BUILD_ID` to an explicit value when needed.
The pinned Rust toolchain is stable 1.85.0. `flake.lock` pins Nix inputs.

## Prepare a microSD card

The script never partitions, formats, mounts, or targets a block device. Create
an MBR or GPT FAT32 partition by your normal system procedure, mount it, verify
the mount path carefully, and pass an **empty directory** at that path:

```sh
./scripts/build.sh
./scripts/prepare-sd.sh /run/media/$USER/PIE8BOOT
sync
```

The script refuses a non-empty destination. It downloads and checksum-verifies
`bootcode.bin`, `start.elf`, `fixup.dat`, and their Broadcom licence from the
official `raspberrypi/firmware` repository at revision
`cd2a1f1b67266d85f8c00dd9c2190fb8f7b48465`. It then installs those files,
`boot/config.txt`, and `build/kernel8.img`. Network-free alternative: manually
copy the same four pinned firmware files plus `boot/config.txt` and
`build/kernel8.img` to the root of an empty FAT boot partition. Do not copy the
Linux `kernel*.img` files from the firmware repository.

The resulting partition contains:

```text
bootcode.bin  start.elf  fixup.dat  LICENCE.broadcom
config.txt    kernel8.img
```

## UART connection and boot

Use a **3.3 V TTL** USB-to-serial adapter. A 5 V or RS-232-level adapter can
damage the Pi. With power disconnected, wire:

- Pi GPIO14/TXD, physical pin 8, to adapter RX.
- Pi GPIO15/RXD, physical pin 10, to adapter TX (optional for this milestone).
- Pi GND, for example physical pin 6, to adapter GND.

Do not connect the adapter's power lead. Open the terminal before powering the
Pi:

```sh
picocom --baud 115200 --databits 8 --parity n --stopbits 1 /dev/ttyUSB0
```

Expected output (build ID varies):

```text
PIE8 bare-metal firmware
version: 0.1.0
build: 0123456789ab
exception level: EL1
CPU core: 0
Rust initialization: OK
```

This repository has not established a physical boot result until a user runs
that procedure on hardware and records it.

## Troubleshooting

- No UART text: confirm a common ground, adapter RX connected to pin 8, 115200
  8N1, a 3.3 V adapter, and that `enable_uart=1` and `core_freq=250` remain in
  `config.txt`.
- Coloured splash or Linux boots: the card contains stale files or the PIE8
  partition is not the boot partition. Re-stage to an empty FAT partition.
- Repeating/garbled characters: check that `core_freq=250` was not removed and
  that hardware flow control is disabled.
- Green activity LED pattern: consult Raspberry Pi's boot diagnostics; first
  checksum the copied firmware files and `kernel8.img` against the build output.
- Permission denied on the serial port: use the host's serial-access group
  (commonly `dialout`) or an appropriate temporary device rule.

## Pi 3 and Zero 2 W notes

Both boards use a Cortex-A53-class BCM2837 family SoC, the `0x3f000000`
peripheral base, the `0x80000` AArch64 kernel load address, and mini UART on
GPIO14/15 under this configuration. The current firmware intentionally disables
device-tree loading, so it does not depend on board DTBs. Board-specific clock,
power, Wi-Fi/Bluetooth, LED, and later peripheral routing still need physical
verification on each model. The Zero 2 W has a smaller thermal/power envelope;
future timing must not assume Pi 3 sustained clocks. Neither board is presently
hardware-qualified by this repository.

## References and licensing

The implementation was written for PIE8 from the Arm architecture manuals, the
BCM2835 ARM Peripherals manual (the BCM2837 peripheral programming model used
here), and official Raspberry Pi boot/UART documentation; it does not copy a
tutorial or third-party startup implementation. Links and exact decisions are
recorded in `docs/architecture.md`. PIE8 source is MIT licensed. Downloaded
Raspberry Pi firmware remains under `LICENCE.broadcom`, which the preparation
script installs beside it.

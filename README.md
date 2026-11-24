# PIE8

PIE8 is a portable bare-metal Rust music tracker project for Raspberry Pi. The
development platform is Raspberry Pi 3 and the intended final board is
Raspberry Pi Zero 2 W. Milestone 002 provides an interactive hardware
diagnostic: UART fault reporting, timer-driven active-low buttons, and a
320×240 RGB565 display path using BCM2837 SPI0 and an ST7789 controller
hypothesis. It is not yet tracker firmware.

## Build and verify

The Nix flake pins stable Rust 1.85.0, `aarch64-unknown-none`, LLVM tools,
formatting/lint tools, and picocom:

```sh
nix develop
./scripts/check.sh
```

Build only:

```sh
nix develop --command ./scripts/build.sh
```

The output is `build/kernel8.img`. The build ID defaults to the current short
Git commit and can be overridden with `PIE8_BUILD_ID`.

## Deploy

Create and mount an empty FAT32 boot partition using your normal system tools.
The preparation script never partitions, formats, or mounts media, and refuses
a non-empty destination:

```sh
./scripts/build.sh
./scripts/prepare-sd.sh /run/media/$USER/PIE8BOOT
sync
```

It installs `kernel8.img`, `config.txt`, and checksum-verified Raspberry Pi
firmware pinned at revision
`cd2a1f1b67266d85f8c00dd9c2190fb8f7b48465`. Updating existing milestone-001
media is intentionally manual: empty its boot partition after preserving any
needed files, then run the same command. The boot configuration is unchanged:
AArch64, fixed 250 MHz VPU core clock for UART/SPI calculation, no device tree.

## Run diagnostics

Read [hardware.md](docs/hardware.md) before applying power. Connect a 3.3 V TTL
serial adapter and start:

```sh
picocom --baud 115200 --databits 8 --parity n --stopbits 1 /dev/ttyUSB0
```

On boot, UART reports version/build, EL, core, generic-counter frequency, SPI
configuration, display initialization, and each startup colour transfer time.
The display cycles red, green, blue, white, and black, then shows uptime, eight
button indicators, total presses, and SPI/display information. Buttons emit
pressed/released UART events with timestamps and completed press duration.

No physical hardware result is claimed by this repository. Record the board
revision, panel markings, observed colours/orientation, UART log, and button
results when testing.

## Layout

- `crates/pie8-core`: portable button state machine and RGB565 graphics.
- `crates/pie8-rpi`: BCM2837 GPIO, mini UART, SPI0, generic timer, and CPU access.
- `firmware`: startup/vectors, board wiring, ST7789 commands, and diagnostics.
- `tools/image-inspect`: host raw-image sanity checking.
- `boot` and `scripts`: boot configuration, build, checks, and safe SD staging.

See [architecture.md](docs/architecture.md), [hardware.md](docs/hardware.md), and
[roadmap.md](docs/roadmap.md).

## Current limitations

- The display controller is unconfirmed. The code sends ST7789 commands and
  cannot identify a controller because prototype MISO is not connected.
- SPI is blocking and polling at an estimated 7.8125 MHz; there is no DMA.
- Button input is polled at approximately 1 ms with 10 ms debounce. Small region
  transfers temporarily extend that interval; details are in the architecture
  document. Interrupts remain masked.
- There is no audio, filesystem, USB, heap, scheduler, multicore application,
  display autodetection, or tracker sequencing.
- Pi 3 and Zero 2 W share the implemented BCM2837 peripheral base and Cortex-A53
  architecture, but this milestone still needs physical validation on both.

PIE8 source is MIT licensed. Downloaded Raspberry Pi boot firmware retains its
Broadcom licence, installed beside the firmware binaries.

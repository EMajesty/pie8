# PIE8

PIE8 is a portable bare-metal Rust music tracker project targeting Raspberry Pi
3 during development and Raspberry Pi Zero 2 W for the eventual device.
Milestone 003 adds cached virtual memory, on-device performance diagnostics,
interactive display colour correction, and a portable 48 kHz stereo audio
processing foundation.

## Verified hardware baseline

Milestone 002 was physically booted on a Raspberry Pi 3. The display initialized,
text rendered, all eight buttons worked, held-state visualization worked, and
press counts advanced. The intended red/green/blue/white startup sequence
appeared cyan/magenta/yellow/black, consistent with display inversion but not
conclusive controller identification. UART was not tested. No milestone-003 MMU,
benchmark, colour-correction, or audio behavior has been physically verified.

## Build and deploy

```sh
nix develop
./scripts/check.sh
./scripts/build.sh
./scripts/prepare-sd.sh /run/media/$USER/PIE8BOOT
sync
```

`build/kernel8.img` is the bootable output. The staging destination must be an
empty mounted FAT partition; the script never formats or mounts media and
refuses a non-empty directory. Boot firmware remains checksum-pinned. The Nix
flake pins stable Rust 1.85.0 and all cross-build tools.

## Diagnostic controls

Every page uses Button 7 for previous and Button 8 for next.

- Button page: shows eight active-low held indicators and total press count.
- Colour page: B1 toggles inversion, B2 toggles RGB/BGR, B3 cycles SPI speed,
  and B4 immediately restores the known-working 7.8125 MHz divider.
- Performance page: B1 measures full/64×64 SPI transfers at 7.8125, 15.625,
  25, and 31.25 MHz, then restores 7.8125 MHz. B2 reruns cached CPU/memory and
  rendering benchmarks. Results are displayed because UART remains untested.
- Audio page: B1 starts/stops generator processing, B2 changes waveform, B3/B4
  adjust frequency, and B5/B6 adjust gain. This exercises processing only and
  intentionally produces no physical sound; the PWM/DMA backend is blocked.

The colour page simultaneously labels red, green, blue, cyan, magenta, yellow,
white, and black. Compare labels with observed colours, try inversion first,
then RGB/BGR. The controller is still only an ST7789 hypothesis.

## Current scope and blockers

The firmware has no Linux runtime, heap, scheduler, filesystem, USB, tracker
sequencing, I²S backend, or working analog-audio backend. It boots silent.

The Pi 3 analog jack is PWM-derived, but a reliable continuous DMA backend is
not implemented because the published BCM2835 peripheral document does not
fully specify the clock-manager programming needed to establish an exact audio
carrier/sample cadence, and authoritative board-specific confirmation of the
internal GPIO40/GPIO45 routing, DMA channel ownership, and SDRAM bus alias is
still required. Guessing those values could produce unstable clocks or corrupt
memory. The UI states this blocker rather than claiming playback.

See [architecture](docs/architecture.md), [hardware and test procedure](docs/hardware.md),
and [roadmap](docs/roadmap.md).

PIE8 source is MIT licensed. Downloaded Raspberry Pi boot firmware retains its
Broadcom licence.

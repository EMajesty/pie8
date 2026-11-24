# Roadmap

## Completed baseline

- Milestone 001: reproducible AArch64 boot image, EL1 startup, UART, SD staging.
- Milestone 002: display/text/buttons physically confirmed on Pi 3.
- Milestone 003: MMU/caches and interactive diagnostics physically confirmed;
  inversion OFF, RGB, and all SPI rates work in the reported test. Benchmarks,
  UART, long-term 31.25 MHz integrity, and Zero 2 W remain unverified.

## Milestone 004 — storage and sample loading

Implemented host-testable foundation:

- Mailbox ARM-memory discovery, dynamic identity map, fallback, and sample arena.
- Strictly read-only block abstraction, MBR/FAT32 traversal, host images.
- PCM16/24 mono/stereo 48 kHz WAV parsing/conversion and sample metadata.
- Reproducible partitioned FAT32/WAV fixtures and safe sample copy tooling.
- Deterministic SDHOST external-card routing, polling initialization, card
  identity/capacity, read-only block access, FAT mount and on-screen retry/error
  diagnostics. Physically confirmed on Pi 3 at 1,897 KiB/s with successful
  FAT32 browsing, WAV loading and waveform display.

Exit work: repeat on Zero 2 W and additional cards, exercise malformed/error
paths, quantify long-duration reliability and polling latency, and consider
native CMD18 now that CMD17 is proven. No write support is planned.

## Milestone 005 — sampler and DSP measurement

- Allocation-free PCM16/24 mono/stereo voices with fractional interpolation,
  source-rate correction, pitch, reverse, ranges and loop modes.
- ADSR/de-click envelope, deterministic fixed-capacity stealing, panning,
  mixing, peak/clipping statistics and generation-safe arena reset.
- Shared host stereo WAV renderer and silent on-device sampler controls.
- DSP matrix for 8/16/32/64/128 voices and 64/128/256-frame blocks.

New target functionality awaits physical testing.

## UI-001 — tracker interaction specification

The LSDJ 9.2.6 and M8 6.5.2 manuals are studied in `docs/ui/`. The proposed
interaction model uses the M8-labelled eight controls, a stable spatial screen
map, shared-object clone safeguards, semantic palette roles and native 320×240
mockups. A future desktop simulator will share its no_std input reducer and
RGB565 renderer with firmware. UI-001 changes no runtime code.

Before production UI work: resolve the decisions listed in
`docs/ui/interaction.md`, prototype the input timing on hardware, validate the
map and mockups with LSDJ/M8 users, and choose a lightweight host presentation
library.

## Next audio milestone

Do not implement Pi 3 PWM. After selecting I²S DAC/codec hardware and resolving the
prototype GPIO18–21 conflict, build a separate continuous DMA backend with
verified bus addressing/coherency, underrun accounting, and clean start/stop.

## Later

Tracker sequencing, instruments, project format, USB MIDI, final carrier PCB,
battery/power management, audio/MIDI I/O qualification, recovery, and updates.

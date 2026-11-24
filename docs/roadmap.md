# Roadmap

## Milestone 001 — bootstrap

Reproducible AArch64 image, single-core EL1 startup, UART, and pinned SD staging.

## Milestone 002 — hardware foundation

Display/text/eight buttons/held visualization/counting physically confirmed on
Pi 3. UART and Zero 2 W remain untested. Colour inversion issue recorded.

## Milestone 003 — runtime performance and initial audio (current)

- EL1 MMU, caches, bounded identity map, Device MMIO, and DMA coherency contract.
- On-screen pre/post-cache CPU/memory/render and multi-speed SPI benchmarks.
- Runtime inversion/RGB/BGR controls and labelled eight-colour test.
- Allocation-free 48 kHz stereo float processing and test generators.
- PWM/DMA analog backend explicitly blocked pending authoritative clock/routing,
  DMA-channel, and bus-address evidence.

Exit evidence: Pi 3 MMU boot, correct colour settings, stable SPI ceiling,
captured benchmark values, audio-generator scheduling stats, UART capture, and
exception-path test. Then repeat safe configuration on Zero 2 W.

## Milestone 004 — DMA audio proof

- Resolve Pi 3 PWM backend blockers or formally retire that temporary route.
- Integrate PCM5102A as a separate I²S/DMA backend after rewiring GPIO18–21.
- Prove continuous 48 kHz stereo, cache ownership, underrun accounting, clean
  start/stop, and display activity during playback on an oscilloscope/audio test.

## Later work

Validated display configuration, storage, tracker sequencing, synthesis and
sampling, final UI/controls, power/thermal behavior, recovery, and updates.

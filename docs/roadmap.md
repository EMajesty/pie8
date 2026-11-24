# Roadmap

## Milestone 001 — bootstrap (current)

- Reproducible AArch64 build and raw Raspberry Pi image.
- Single-core EL1 startup, stack and BSS initialization.
- Polling mini-UART boot diagnostics.
- Safe, pinned boot-partition staging and host verification.
- Physical boot validation remains outstanding.

## Milestone 002 — board foundation

- Exception vectors, timer, monotonic time, and explicit fault diagnostics.
- Board identity/peripheral discovery and a deliberate device-tree policy.
- GPIO input abstraction and debounced physical control prototype.
- Pi 3 and Zero 2 W hardware-in-the-loop smoke tests.

## Milestone 003 — display proof

- Confirm the 320×240 panel controller and electrical interface.
- SPI transport, reset/backlight control, framebuffer format, and dirty-region
  update measurements. No UI framework until measured constraints are known.

## Milestone 004 — deterministic audio proof

- Select and validate the I²S audio hardware.
- Fixed 48 kHz stereo clocking, DMA ring, underrun reporting, and a generated
  test tone with latency/jitter measurements.

## Later work

- Read-only then crash-safe writable microSD storage.
- Tracker data model, sequencing, synthesis/sampling, controls, and UI.
- Power, thermal, startup-time, recovery, and update design.

Each milestone must retain host tests and reproducible builds. Hardware claims
must name the board revision and test setup that produced the evidence.

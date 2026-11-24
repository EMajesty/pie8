# Roadmap

## Milestone 001 — bootstrap (complete in software)

Reproducible AArch64 image, single-core EL1 startup, BSS/stack, UART diagnostics,
and pinned SD staging. Physical validation remains unrecorded.

## Milestone 002 — hardware foundation (current)

- FP/SIMD-safe EL transition, exception vectors/fault UART, ordered MMIO, timer.
- Central prototype wiring, active-low debounced buttons, SPI0, and ST7789 path.
- Static RGB565 graphics and interactive regional-update diagnostic dashboard.
- Host tests and ELF layout assertions.

Exit evidence still required: Pi 3 and Zero 2 W UART logs; display controller
marking/colour/orientation/offset results; measured full and regional transfer
times; all individual/simultaneous button tests.

## Milestone 003 — validated display and input foundation

- Resolve controller identity and commit measured panel configuration.
- Tune SPI from hardware evidence; evaluate DMA only after profiling.
- Reconcile GPIO18–21 prototype conflicts with the I²S hardware plan.
- Add repeatable hardware-in-the-loop smoke tests and exception injection test.

## Milestone 004 — deterministic audio proof

- Select I²S audio hardware and final pin allocation.
- Fixed 48 kHz stereo clocking, DMA ring, underrun diagnostics, and generated
  test tone with measured latency/jitter.

## Later work

Crash-safe microSD storage, tracker data/sequencing, synthesis and sampling,
final controls/UI, power/thermal design, recovery, and updates. No milestone may
claim hardware behavior without naming the board, wiring, and captured evidence.

# Prototype hardware and validation

## Physical validation record

Reported physical test on Raspberry Pi 3 using milestone-002 commit
`b10e10a3a86c8853be8aa194a8b6db10fbf213e7`:

Confirmed:

- Bare-metal firmware booted.
- Display initialization and bitmap text rendering worked.
- All eight buttons, held-state visualization, and press counting worked.

Observed:

- Intended red, green, blue, white, black screens appeared cyan, magenta,
  yellow, and black. This suggests inversion but does not prove controller type.

Not tested:

- UART output.
- Milestone-003 MMU/cache operation, benchmarks, runtime colour changes, faster
  SPI clocks, audio processing timing, or any physical audio output.
- Raspberry Pi Zero 2 W.

## Prototype wiring

Software uses BCM numbering; physical numbers are the 40-pin header.

| Function | BCM | Physical | Function | BCM | Physical |
|---|---:|---:|---|---:|---:|
| LCD SCK | 11 | 23 | Button 1 | 21 | 40 |
| LCD MOSI | 10 | 19 | Button 2 | 20 | 38 |
| LCD CE0 | 8 | 24 | Button 3 | 26 | 37 |
| LCD DC | 25 | 22 | Button 4 | 16 | 36 |
| LCD RESET | 24 | 18 | Button 5 | 19 | 35 |
| LCD BACKLIGHT | 18 | 12 | Button 6 | 13 | 33 |
| UART TX | 14 | 8 | Button 7 | 12 | 32 |
| UART RX | 15 | 10 | Button 8 | 6 | 31 |

Button inputs use internal pull-ups and short to ground when pressed. LCD power
is 3.3 V. Backlight GPIO18 is logic control, not a power source. GPIO18–21
conflict with future native I²S and remain unchanged for this prototype.

## Milestone-003 physical test

1. Keep the known-working milestone-002 card available for recovery. Prepare a
   separate empty FAT partition with the commands in the README.
2. Power the Pi 3 with the existing display/buttons wiring. Confirm the colour
   page appears; failure before it may indicate MMU/cache initialization.
3. Confirm all eight labelled swatches. If labels match, leave inversion OFF and
   RGB. Otherwise press B1 for inversion, then B2 for RGB/BGR, and record the
   exact combination and photograph.
4. Press B3 to try 15.625, 25, and 31.25 MHz. At each speed inspect edges/text
   for corruption. Press B4 to return to 7.8125 MHz immediately.
5. Use B8 to reach Performance. Press B1 once, wait for all four full-screen
   transfers, and photograph/transcribe every result. Confirm it restores the
   safe SPI rate. Press B2 and record CPU/memory/render results.
6. Navigate to Button Test and repeat individual and simultaneous presses,
   noting that B7/B8 navigate pages.
7. Navigate to Audio. Confirm it clearly says no output/backend blocked. Exercise
   waveform/frequency/gain and generator start/stop; verify block and late counts
   change. Expect silence from the jack.
8. If a 3.3 V TTL adapter is available, connect TX pin 8 to adapter RX and common
   ground only, open 115200 8N1, and capture boot/benchmark/button logs. Never use
   RS-232 or 5 V levels and do not connect adapter power.
9. Power-cycle and repeat the working colour/SPI settings on Zero 2 W later; do
   not infer that result from Pi 3.

## Troubleshooting

- Blank immediately after update: restore the prior card, capture UART if
  possible, and report whether an exception line contains ESR/ELR/FAR.
- Complementary colours: toggle inversion. Red/blue swapped: toggle RGB/BGR.
- Shift/mirroring: retain documented offsets/orientation testing from milestone
  002; controller identity is still unconfirmed without MISO.
- Corruption only at higher SPI speed: press B4 and record the highest stable
  calculated speed. Faster clocks are experiments, not new defaults.
- No analog audio: expected. The PWM/DMA backend is intentionally absent.

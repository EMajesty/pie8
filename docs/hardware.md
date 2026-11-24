# Hardware, validation, and test procedure

## Validation record

Physical Raspberry Pi 3 testing through milestone 003 confirms:

- Bare-metal boot, display initialization/text, all eight buttons, held-state
  visualization, press counting, and interactive diagnostic pages.
- Correct display colours with inversion **OFF** and **RGB** order.
- All selectable 7.8125, 15.625, 25, and 31.25 MHz SPI settings appeared to
  work while tested.
- Existing display/button behavior remained operational with MMU/caches.

This does not qualify 31.25 MHz signal integrity, temperature/voltage margins,
or long-duration stability. Quantitative benchmarks and UART remain unrecorded.
Mailbox memory discovery, storage pages, FAT/WAV loading, and Zero 2 W remain
unverified on hardware.

## Prototype wiring

Software uses BCM numbering; physical numbers refer to the 40-pin header.

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

Buttons short to ground and use pull-ups. LCD supply is 3.3 V; backlight GPIO18
is logic only. GPIO18–21 conflict with future I²S and remain unchanged now.

## Future carrier direction

The Zero 2 W carrier is planned to combine USB-C power/charging/data/USB MIDI,
rechargeable battery, custom I²S DAC or codec, stereo headphone/line output,
TRS Type A MIDI input, and possibly MIDI output/stereo input. No electrical
design or pin allocation is finalized. Pi 3 PWM audio is explicitly deprioritized.

## Milestone 004 physical test

1. Keep the working milestone 003 card. Update a separate mounted boot FAT
   partition using `prepare-sd.sh`; optionally copy fixtures with
   `install-samples.sh`. Neither script formats media.
2. Boot the Pi 3. Confirm Colours initially reports inversion OFF, RGB, and
   31250 kHz. Recheck all colours/buttons before testing storage pages.
3. Navigate with B8 to Storage. Photograph the detected ARM RAM and mapped MiB.
   It must be at least 16 MiB and plausible for the firmware memory split.
   A zero detected value with 16 MiB mapped means mailbox fallback.
4. Confirm Storage explicitly reports `SD: NOT INITIALIZED` and routing error.
   This is the expected milestone result, not an SD-card failure.
5. Navigate to Samples. Confirm the arena capacity is nonzero and the page says
   physical SD unavailable. Load/release buttons are intentionally inactive.
6. Re-run Performance and record every numerical result. Verify the display
   returns to 31.25 MHz; B4 on Colours must restore 7.8125 MHz at any time.
7. Exercise the audio processing page while repeatedly changing pages. Physical
   audio remains unavailable and silence is expected.
8. For UART, use only a 3.3 V TTL adapter: Pi TX physical 8 to adapter RX and
   common ground, 115200 8N1. Never attach RS-232/5 V or adapter power.
9. Repeat on Zero 2 W only after preserving a recovery card. Record detected RAM
   and never infer compatibility from the Pi 3 result.

If boot fails after the update, restore milestone 003 and capture any UART
ESR/ELR/FAR line. If memory is implausible, report both detected/mapped values
before attempting storage work. No destructive SD diagnostic exists.

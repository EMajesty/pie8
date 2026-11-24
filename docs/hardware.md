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
Milestone 004B physical Raspberry Pi 3 testing additionally confirms:

- SDHOST reaches Ready with the external boot card on GPIO48--53 ALT0.
- The 29.7 GiB SDHC card capacity is reported as 30,436 MiB.
- MBR FAT32 partition type `0x0c` is found at LBA 2048 and mounts successfully.
- The diagnostic sequential CMD17 test measured **1,897 KiB/s** with the
  four-bit bus, 25 MHz requested clock, and full-divider mode enabled.
- Final `HSTS` was `0x00000000`; directory browsing, WAV loading, sample-arena
  storage and waveform display worked on-device.

UART and Zero 2 W remain unverified. This validates one Pi 3/card combination,
not hot-plug, broad card compatibility, long-duration reliability, native
CMD18, or signal-integrity margins.

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

## Milestone 004B physical SD test

1. Preserve the known-working card image. If the card already has the milestone
   004 bundle and unchanged `config.txt`, mount its FAT boot partition and copy
   only `build/kernel8.img` over `kernel8.img`. A new/unknown card needs the
   complete pinned `prepare-sd.sh` procedure. Neither operation formats media.
2. Generate and install fixtures without deleting unrelated files:

   ```sh
   ./scripts/generate-storage-fixtures.sh /tmp/pie8-fixtures
   ./scripts/install-samples.sh /run/media/$USER/PIE8BOOT /tmp/pie8-fixtures/samples/*
   sync
   ```

3. Boot the Pi 3. Confirm Colours initially reports inversion OFF, RGB, and
   31250 kHz. Recheck all colours/buttons before testing storage pages.
4. Navigate to Storage. It must show `SDHOST 3F202000 GPIO48-53 ALT0`, a
   nonzero board revision, `READY`, plausible card capacity, a FAT32 mount and
   nonzero read throughput. Photograph all values. If it fails, photograph the
   stage, last command/response and HSTS; press B1 once to perform a complete
   retry without rebooting and record whether the values change.
5. Navigate to Samples. B3/B4 select entries, B5 enters a directory and B6
   returns to `/samples`. Select and B1-load each supported fixture. Confirm
   bit depth, channels, 48000 Hz, duration, bytes, load time and waveform.
   B2 must release the loaded arena. The 44.1 kHz fixture must report
   `UNSUPPORTED WAV`, never load or play at the wrong rate.
6. Load the longer fixture, release it, and load another file. Repeat directory
   enumeration after an SD retry. Confirm button/display behavior remains
   responsive. Firmware performs only reads; do not remove power while the host
   computer is writing fixtures.
7. Re-run Performance and record every numerical result. Verify the display
   returns to 31.25 MHz; B4 on Colours restores 7.8125 MHz.
8. For UART, use only a 3.3 V TTL adapter: Pi TX physical 8 to adapter RX and
   common ground, 115200 8N1. Never attach RS-232/5 V or adapter power.
9. Repeat on Zero 2 W with a recovery card. Expect the same SDHOST/GPIO route;
   record board revision, capacity, FAT result and throughput independently.
If boot fails after the update, restore the prior kernel and capture any UART
ESR/ELR/FAR line. If Storage fails, report the exact on-screen raw values before
changing configuration. No destructive SD diagnostic exists.

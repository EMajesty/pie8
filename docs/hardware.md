# Prototype hardware

All software uses Broadcom (BCM) GPIO numbering. Physical numbers refer to the
40-pin Raspberry Pi header.

| Function | BCM GPIO | Physical pin | Configuration |
|---|---:|---:|---|
| LCD SCK | 11 | 23 | SPI0 SCLK, ALT0 |
| LCD MOSI | 10 | 19 | SPI0 MOSI, ALT0 |
| LCD CS | 8 | 24 | SPI0 CE0, ALT0 |
| LCD DC | 25 | 22 | output |
| LCD RESET | 24 | 18 | output, active low |
| LCD BACKLIGHT | 18 | 12 | logic output only |
| Button 1 | 21 | 40 | input, internal pull-up |
| Button 2 | 20 | 38 | input, internal pull-up |
| Button 3 | 26 | 37 | input, internal pull-up |
| Button 4 | 16 | 36 | input, internal pull-up |
| Button 5 | 19 | 35 | input, internal pull-up |
| Button 6 | 13 | 33 | input, internal pull-up |
| Button 7 | 12 | 32 | input, internal pull-up |
| Button 8 | 6 | 31 | input, internal pull-up |
| UART TX | 14 | 8 | mini UART ALT5 |
| UART RX | 15 | 10 | mini UART ALT5 |

The LCD supply is connected to 3.3 V and ground. The backlight GPIO is a
logic-level enable, **not a power source**; use the module's intended backlight
power/driver circuit. Every button shorts its listed GPIO to ground when held,
so firmware treats low as pressed. Button numbering is provisional and implies
no tracker function.

GPIO18, 19, 20, and 21 conflict with the PCM/I²S signals needed for future
native audio. They are intentionally retained as the actual prototype
backlight/button wiring for this milestone. The hardware must be revised or an
alternative audio/control arrangement chosen before I²S work.

## UART safety

Use a 3.3 V TTL adapter, never 5 V TTL or RS-232 voltage levels. With all power
off, connect Pi pin 8/TX to adapter RX, optionally pin 10/RX to adapter TX, and
a Pi ground such as pin 6 to adapter ground. Do not connect the adapter power
lead. Open 115200 8N1 with hardware/software flow control disabled.

## Display assumption

The module is advertised as a 3.2-inch 320×240 “ST7789/ILI9488” SPI TFT. PIE8
currently assumes ST7789: RGB565, 240×320 internal address space rotated into
320×240 landscape, RGB order, inversion enabled, and zero address offsets.
There is no MISO connection in the prototype wiring, so controller ID reads are
not evidence and are not attempted. SPI starts at 7.8125 MHz.

## Diagnostic test procedure

1. Recheck every connection against the table with power removed. Confirm the
   display is a 3.3 V-compatible module and its backlight is correctly powered.
2. Prepare the boot partition as described in the README and attach UART.
3. Start the serial terminal, then power the Pi. Save the complete log.
4. Confirm UART reaches `Rust initialization: OK`, reports SPI configuration,
   and says that ST7789 initialization was sent.
5. Observe red, green, blue, white, and black screens in that order. Record the
   reported transfer microseconds and any swapped colors, inversion, clipping,
   mirroring, or shift.
6. Confirm the dashboard is landscape and fully visible.
7. Hold each button separately. Its numbered indicator must turn green while
   held, return dark on release, increment total presses once, and produce one
   press plus one release UART line with a plausible duration.
8. Hold several buttons together and confirm all indicators/events operate.
9. Leave the unit running and confirm uptime advances without UART faults.
10. Repeat on Pi 3 and Zero 2 W before making compatibility claims.

## Display troubleshooting

- **Blank, no backlight:** verify 3.3 V/ground and the module's backlight driver.
  GPIO18 is only enable logic. Confirm RESET/DC/CE0 pins and inspect UART for an
  SPI timeout.
- **Backlight on, all white/black:** verify SCK, MOSI, DC, RESET, and CE0. If the
  colour test never appears but UART says commands were sent, the controller is
  likely not ST7789-compatible or CS/DC polarity/wiring is wrong.
- **Red and blue swapped:** change `DISPLAY_CONFIG.color_order` between `Rgb`
  and `Bgr` in `firmware/src/board.rs`, rebuild, and repeat the colour test.
- **Colors appear as negatives:** toggle `DISPLAY_CONFIG.inverted` and retest.
- **Mirrored/rotated:** choose the appropriate `Orientation` in the same config.
- **Image shifted or clipped:** determine the module's hidden column/row origin
  and set `x_offset`/`y_offset`. Test every edge before accepting the values.
- **Still incompatible:** inspect controller markings or seller schematics. An
  ILI9488 is typically a 320×480-class controller and often needs different
  initialization/pixel handling. Capture that evidence before adding a driver.
- **Unstable at higher SPI speed:** restore divider 32. Smaller even dividers are
  faster; only increase speed after the conservative configuration is reliable.

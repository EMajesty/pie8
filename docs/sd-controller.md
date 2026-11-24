# SD controller selection and bring-up

## Decision

PIE8 uses the BCM2835 **SDHOST** controller at ARM physical address
`0x3f20_2000` for the external boot microSD card on Raspberry Pi 3B, 3B+ and
Zero 2 W. Firmware explicitly routes BCM GPIO48--53 to ALT0 before resetting
the controller:

| Signal | BCM GPIO | Function |
|---|---:|---|
| SD clock | 48 | ALT0 |
| SD command | 49 | ALT0 |
| SD data 0--3 | 50--53 | ALT0 |

The Arasan controller at `0x3f30_0000` and its GPIO34--39 ALT3 route are left
alone. On these boards that path serves the soldered Wi-Fi SDIO device. PIE8
never probes or initializes both controllers and does not touch the prototype
display/button header pins.

This is an explicit board profile, not a board-revision guess. The firmware
revision property is queried and displayed for diagnosis, but it does not
change the routing. All currently supported boards share this routing.

## Evidence

- Raspberry Pi's downstream Linux device trees describe `sdhost_pins` as
  GPIO48--53 function 4 (ALT0), and the Wi-Fi `sdio_pins` as GPIO34--39
  function 7 (ALT3). The Pi 3B and 3B+ board files enable `sdhost`; the
  [Zero 2 W device tree](https://github.com/raspberrypi/linux/blob/rpi-6.18.y/arch/arm/boot/dts/broadcom/bcm2710-rpi-zero-2-w.dts)
  uses the same external-card route.
- The upstream Linux
  [BCM2835 SDHOST driver](https://github.com/torvalds/linux/blob/master/drivers/mmc/host/bcm2835.c)
  documents the register layout, divider behavior, FIFO state machine and the
  silicon FIFO-threshold restriction used during this independent
  implementation.
- The upstream Linux
  [Arasan SDHCI driver](https://github.com/torvalds/linux/blob/master/drivers/mmc/host/sdhci-iproc.c)
  and Raspberry Pi device trees confirm that it is a distinct controller and
  not the selected external-card path for this profile.
- The [Circle bare-metal framework](https://github.com/rsta2/circle) supports
  SDHOST on Pi 1--3-class hardware and was used to cross-check reset and
  bring-up sequencing.
- Raspberry Pi's
  [mailbox property interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
  defines the board-revision property tag used only for diagnostics.
- Raspberry Pi's
  [configuration documentation](https://www.raspberrypi.com/documentation/computers/config_txt.html)
  documents boot configuration and device-tree behavior.

Linux code is GPL-2.0 and Circle is GPL-3.0. No implementation code was copied
from either project. PIE8 uses the MIT-licensed code in this repository,
written from register/protocol behavior and checked against those references.

## Boot firmware and device tree

The existing PIE8 profile deliberately uses `device_tree=` and fixes
`core_freq=250`. The SDHOST driver therefore does not depend on an FDT pointer,
aliases, overlays, firmware pin mux state, or a controller left initialized by
the bootloader. It establishes the known board route and resets/powers the
controller itself.

Re-enabling the firmware device tree would add parser and boot-bundle
dependencies without resolving an ambiguity for the supported boards. The
`sdhost` overlay is also inappropriate here: overlays can exchange SD/MMC
routes and can disable Wi-Fi. PIE8's explicit profile is smaller and
deterministic.

Once a boot card has the milestone 004 boot bundle and unchanged `config.txt`,
replacing `kernel8.img` alone is sufficient for 004B. For a new card, use the
pinned full staging procedure in the README.

## Initialization and reads

The driver:

1. configures GPIO48--53, powers down and resets SDHOST registers;
2. applies the hardware-required FIFO thresholds, powers up, and selects an
   actual 400 kHz identification clock from the fixed 250 MHz core clock;
3. performs CMD0, CMD8, ACMD41, CMD2, CMD3, CMD9 and CMD7;
4. validates the interface response, OCR ready/CCS bits, RCA and CSD capacity;
5. selects four-bit width with ACMD6 and switches to 25 MHz;
6. issues read-only CMD17 transfers through the FIFO with generic-counter
   deadlines and controller error capture.

The `ReadBlockDevice::read_blocks` implementation currently satisfies
multi-block requests by a bounded sequence of CMD17 operations. Native CMD18
is intentionally deferred until the single-block state machine has physical
validation. There are no card write, erase, lock or destructive commands.

## Kernel-entry assumptions

The VideoCore firmware must read the card to load PIE8, but PIE8 does not rely
on the firmware's controller, clock, card-selection or pin state at kernel
entry. It relies only on the documented 250 MHz core clock selected in
`config.txt`, the BCM2837 peripheral base, and the board wiring above.

## Diagnostics and remaining uncertainty

The Storage page shows the raw board revision, selected controller/routing,
initialization stage, capacity, last command/response, SDHOST status, FAT32
mount result and measured sequential read rate. Button 1 retries a complete
reset and initialization without rebooting.

The implementation has been build- and host-tested but has not yet been run
against a physical controller. In particular, command-busy completion and the
SDHOST FIFO transition need hardware confirmation. Keep a photo or exact copy
of the displayed stage, command, response and `HSTS` value when reporting a
failure.

# Architecture and technical decisions

## Milestone 001 execution path

The VideoCore boot firmware loads the flat `kernel8.img` at physical address
`0x80000` in AArch64 mode. `_start` is the first byte in the image. Every core
may arrive there, so assembly reads `MPIDR_EL1` before using RAM: affinity-0
core 0 continues and all others remain in a `wfe` loop. The primary core:

1. Detects `CurrentEL`.
2. If necessary, transitions EL3 to non-secure AArch64 EL2, then EL2 to EL1h.
3. Masks exceptions, disables translation/caches, installs a 64 KiB aligned
   stack, and zeroes the linker-defined BSS range in eight-byte units.
4. Calls `kernel_main`, which initializes GPIO14/15 and mini UART, prints the
   banner, then waits forever.

The linker places boot code first at `0x80000`, followed by text, read-only data,
data, BSS, and the boot stack. BSS and stack are `NOLOAD`, so `objcopy` does not
inflate the image with zeroes. The firmware has no heap and no interrupts.

## Crate boundaries

`pie8-core` owns only portable values and transformations; its tests run on the
host. `pie8-rpi` owns documented BCM2837 register access and AArch64 system
register operations. `pie8-firmware` owns startup policy and product diagnostics.
The image inspector is a real host tool rather than a placeholder: it rejects
empty or all-zero deployment images. New crates should be added only when a
similar concrete boundary appears.

## UART decision

UART1 (the 16550-like mini UART) is used on both target boards because firmware
normally routes it to GPIO14/15 while PL011 is used for Bluetooth. Its clock is
derived from the VPU core clock. `enable_uart=1` and `core_freq=250` fix that
clock at 250 MHz; divisor 270 gives `250000000 / (8 * (270 + 1))`, approximately
115200 baud. The driver uses polling, 8 data bits, no parity, one stop bit, and
no flow control. MMIO access is volatile, and every unsafe block states its
invariant. The fixed clock can increase idle power and is an accepted bootstrap
tradeoff. A later mailbox-derived clock or PL011 policy may replace it.

## Boot firmware and reproducibility

The SD staging script pins official Raspberry Pi firmware commit
`cd2a1f1b67266d85f8c00dd9c2190fb8f7b48465` and SHA-256 checks each downloaded
file. Nix inputs, Rust version, compilation target, and release settings are
pinned. The default diagnostic build ID comes from Git and therefore identifies
source, while identical source and tool inputs produce the same image. Override
`PIE8_BUILD_ID` explicitly for release automation.

## Significant decisions and open questions

- Target EL is EL1h, leaving EL2 available to firmware and making future OS-like
  facilities conventional. Exception vectors are not installed yet, so DAIF
  remains masked.
- Only affinity-0 core 0 runs. Multicore release/mailboxes are deferred.
- `device_tree=` avoids passing or parsing board descriptions during bootstrap.
  Re-enable DT before drivers need board-specific discovery.
- Peripheral base `0x3f000000` is shared by Pi 3 and Zero 2 W. Supporting Pi 4/5
  requires runtime discovery or a separate platform target.
- Confirm the actual TFT controller, wiring, SPI ceiling, and orientation.
- Decide the audio clock master, codec/DAC, DMA channel strategy, and underrun
  recovery before audio work.
- Decide whether future firmware updates keep the legacy GPU firmware path or
  use a separately managed boot bundle.

## Primary references

- [Raspberry Pi configuration and UART documentation](https://www.raspberrypi.com/documentation/computers/configuration.html)
- [Raspberry Pi `config.txt` documentation](https://www.raspberrypi.com/documentation/computers/config_txt.html)
- [Raspberry Pi boot-folder documentation](https://www.raspberrypi.com/documentation/computers/configuration.html#boot-folder-contents)
- [BCM2835 ARM Peripherals](https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf)
- [Arm A-profile exception model](https://developer.arm.com/documentation/102412/latest/)
- [Arm A64 instruction and system-register reference](https://developer.arm.com/documentation/ddi0602/latest/)
- [Official Raspberry Pi firmware repository and licensing](https://github.com/raspberrypi/firmware)

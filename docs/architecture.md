# Architecture and technical decisions

## Startup and exception state

The GPU firmware loads `_start` at physical `0x80000`. All nonzero MPIDR
affinity-0 cores park before touching the single boot stack. The primary core
normalizes EL3 or EL2 entry to EL1h. EL3 selects non-secure AArch64 and clears
`CPTR_EL3`; EL2 sets `HCR_EL2.RW`, clears `CPTR_EL2.TFP`, permits EL1 physical
counter/timer access through `CNTHCTL_EL2`, and clears `CNTVOFF_EL2`. At EL1,
startup preserves required `SCTLR_EL1` RES1 fields with translation/caches off,
sets `CPACR_EL1.FPEN=0b11`, installs `VBAR_EL1`, initializes the stack and BSS,
then enters Rust. DAIF remains masked because this milestone is polling-only.

The AArch64 vector section is 2048-byte aligned and has all 16 architected
128-byte entries. Every unexpected synchronous exception, IRQ, FIQ, or SError
prints its vector number plus `ESR_EL1`, `ELR_EL1`, `FAR_EL1`, and `SPSR_EL1`
then executes a completion barrier and parks. It does not attempt unsafe
recovery. `FAR_EL1` is printed even though its architectural validity depends
on the ESR exception class and FnV bit.

## Memory layout and unsafe invariants

Code, the vector table, read-only data, data, and BSS are followed by a
64-byte-aligned 153,600-byte framebuffer and a 64 KiB stack. Framebuffer and
stack are `NOLOAD`; they do not inflate `kernel8.img`. Linker assertions check
vector size/alignment and framebuffer/stack separation. A synchronized
`UnsafeCell` owns the framebuffer. Its safety invariant is that startup has
permanently parked all other cores and temporary framebuffer borrows never
overlap in the core-0 event loop.

MMIO reads/writes are volatile and bracketed by `dmb osh`; a `dsb sy` precedes
terminal fault waits. This is deliberately conservative with the MMU disabled.
GPIO pull configuration uses generic-counter delays rather than CPU loop timing.

## Time and buttons

The monotonic timer reads `CNTFRQ_EL0` and `CNTPCT_EL0`. Conversion splits
quotient/remainder to avoid overflow. Microsecond delay rounds up to the next
counter tick; millisecond delay builds on it. Timestamp differences use wrapping
subtraction.

The portable eight-button state machine consumes logical pressed samples, so
active-low conversion stays in board code. Each button independently tracks a
candidate, stable state, transition time, and press start. The default debounce
is 10 ms. One update can return simultaneous events for all eight buttons.

The application targets a 1 ms poll interval. A button-indicator update sends
only 10×22 pixels: 440 bytes, theoretically about 0.45 ms on the wire at
7.8125 MHz, plus commands and MMIO overhead. Uptime and press-count regions can
take a few milliseconds. Therefore presses that remain stable for roughly
15 ms should be observable in normal operation; pulses close to or below the
10 ms debounce threshold are intentionally not guaranteed. Exact transfer and
poll latency is unmeasured until hardware testing. Startup full-screen colour
tests block polling by design because the diagnostic loop has not started.

## SPI and display

BCM2837 SPI0 owns GPIO8, GPIO10, and GPIO11 in ALT0, mode 0, hardware CE0, and an even clock
divider. Divider 32 with the boot-configured 250 MHz core clock is an estimated
7.8125 MHz. Each blocking transfer drains RX while filling TX so an unconnected
MISO cannot fill and stall the receive FIFO. Transfers have a 100 ms timeout,
always deassert transfer-active state on failure, and RGB565 bulk output uses a
small fixed byte-order conversion buffer. DMA is deferred.

ST7789 commands are isolated from SPI transport. Configuration exposes four
orientations, RGB/BGR order, inversion, and X/Y offsets. The prototype defaults
to landscape 320×240, RGB, inversion enabled, and zero offsets. Initialization
uses hardware reset, sleep-out, RGB565 `COLMOD`, `MADCTL`, inversion, normal
mode, and display-on. Window updates use `CASET`, `RASET`, and `RAMWR`. The
backlight GPIO is only a logic control.

The panel advertisement also mentions ILI9488, whose usual native geometry and
pixel-format behavior differ. Without wired MISO, controller ID reads are not
reliable; visual tests and panel markings are the evidence. A second driver is
deferred until that evidence exists.

## Graphics and application

The portable graphics surface provides RGB565 colors, clipped rectangles,
horizontal/vertical lines, and allocation-free 5×7 bitmap text. The diagnostic
draws into one static framebuffer. Full-screen writes occur only during colour
tests and initial dashboard paint. Uptime, count, and button state use bounded
regions. UART remains independently useful when display initialization fails.

## Performance evidence

Theoretical minimum full-frame payload time is 153,600 bytes × 8 /
7,812,500 ≈ 157 ms. Command overhead, per-row/per-chunk CE transitions, volatile
MMIO, and barriers make real time longer. Firmware measures each full colour
transfer with the generic counter and displays/logs the latest value. No
measured hardware value is recorded yet, so every numeric performance statement
above is an estimate except ELF/image sizes produced by host verification.

## Primary references

- [BCM2835 ARM Peripherals (GPIO and SPI programming model)](https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf)
- [Arm A-profile exception model](https://developer.arm.com/documentation/102412/latest/)
- [Arm generic timer guide](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/Generic%20Timer.pdf)
- [Arm A64 instruction/system-register reference](https://developer.arm.com/documentation/ddi0602/latest/)
- [ST7789V specification](https://dl.espressif.com/dl/schematics/ST7789V_SPEC_V1.0.pdf)
- [Raspberry Pi UART/configuration documentation](https://www.raspberrypi.com/documentation/computers/configuration.html)

The implementation was written specifically for PIE8 from these references; no
third-party driver or tutorial source was copied.

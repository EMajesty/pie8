#![no_main]
#![no_std]

mod board;
mod display;

use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
    panic::PanicInfo,
};

use display::St7789;
use pie8_core::{
    RUST_READY_MESSAGE,
    buttons::{ButtonEventKind, Buttons},
    core_id, exception_level,
    graphics::{Rect, Rgb565, Surface},
};
use pie8_rpi::{
    cpu,
    gpio::{Function, Pull},
    mini_uart::MiniUart,
    spi::Spi0,
    timer::Timer,
};

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const FULL_SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: WIDTH as u32,
    height: HEIGHT as u32,
};

#[repr(align(64))]
struct Framebuffer(UnsafeCell<[u16; WIDTH * HEIGHT]>);

// SAFETY: Startup permanently parks every core except core 0. All framebuffer
// access is serialized by that core's single-threaded diagnostic loop.
unsafe impl Sync for Framebuffer {}

#[unsafe(link_section = ".framebuffer")]
static FRAMEBUFFER: Framebuffer = Framebuffer(UnsafeCell::new([0; WIDTH * HEIGHT]));

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("vectors.S"));

#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    board::assert_wiring();
    let current_el = cpu::current_el();
    let mpidr = cpu::mpidr();
    let timer = Timer::new();
    let mut uart = MiniUart::new();

    writeln!(uart, "PIE8 hardware diagnostics").ok();
    writeln!(uart, "version: {}", env!("CARGO_PKG_VERSION")).ok();
    writeln!(uart, "build: {}", env!("PIE8_BUILD_ID")).ok();
    writeln!(uart, "exception level: EL{}", exception_level(current_el)).ok();
    writeln!(uart, "CPU core: {}", core_id(mpidr)).ok();
    writeln!(uart, "counter frequency: {} Hz", timer.frequency_hz()).ok();
    writeln!(uart, "{RUST_READY_MESSAGE}").ok();

    for pin in board::BUTTONS {
        pin.set_function(Function::Input);
        pin.set_pull(Pull::Up, &timer);
    }

    let spi = Spi0::new(&timer, board::SPI_CLOCK_DIVIDER, board::SPI_TIMEOUT_US);
    let spi_hz = spi.estimated_hz(board::CORE_CLOCK_HZ);
    writeln!(
        uart,
        "SPI0: mode 0, CE0, divider {}, estimated {} Hz",
        spi.divider(),
        spi_hz
    )
    .ok();
    let mut display = St7789::new(
        spi,
        board::LCD_DC,
        board::LCD_RESET,
        board::LCD_BACKLIGHT,
        board::DISPLAY_CONFIG,
    );
    let display_ok = match display.init(&timer) {
        Ok(()) => {
            writeln!(uart, "display: ST7789 initialization sent").ok();
            true
        }
        Err(error) => {
            writeln!(uart, "display initialization failed: {error:?}").ok();
            false
        }
    };

    let mut full_transfer_us = 0;
    if display_ok {
        for (name, color) in [
            ("RED", Rgb565::RED),
            ("GREEN", Rgb565::GREEN),
            ("BLUE", Rgb565::BLUE),
            ("WHITE", Rgb565::WHITE),
            ("BLACK", Rgb565::BLACK),
        ] {
            framebuffer_surface().clear(color);
            let started = timer.micros();
            if let Err(error) = display.update_region(&timer, framebuffer(), WIDTH, FULL_SCREEN) {
                writeln!(uart, "display colour test failed on {name}: {error:?}").ok();
                break;
            }
            full_transfer_us = timer.micros().wrapping_sub(started);
            writeln!(
                uart,
                "display colour test: {name}, transfer {full_transfer_us} us"
            )
            .ok();
            timer.delay_ms(180);
        }
    }

    draw_dashboard(current_el, spi_hz, full_transfer_us);
    if display_ok {
        report_display_error(
            &mut uart,
            display.update_region(&timer, framebuffer(), WIDTH, FULL_SCREEN),
        );
    }

    let mut buttons = Buttons::<8>::default();
    let mut total_presses = 0_u64;
    let mut last_uptime_second = u64::MAX;
    let mut next_poll_ms = timer.millis();

    loop {
        let now_ms = timer.millis();
        if now_ms.wrapping_sub(next_poll_ms) as i64 >= 0 {
            next_poll_ms = now_ms.wrapping_add(1);
            let samples = board::BUTTONS.map(|pin| !pin.level());
            for event in buttons.update(samples, now_ms).into_iter().flatten() {
                match event.kind {
                    ButtonEventKind::Pressed => {
                        total_presses = total_presses.wrapping_add(1);
                        writeln!(
                            uart,
                            "button {} pressed at {} ms",
                            event.index + 1,
                            event.timestamp_ms
                        )
                        .ok();
                    }
                    ButtonEventKind::Released => {
                        writeln!(
                            uart,
                            "button {} released at {} ms (held {} ms)",
                            event.index + 1,
                            event.timestamp_ms,
                            event.duration_ms
                        )
                        .ok();
                    }
                }
                draw_button(event.index, buttons.is_pressed(event.index));
                draw_press_count(total_presses);
                if display_ok {
                    report_display_error(
                        &mut uart,
                        display.update_region(
                            &timer,
                            framebuffer(),
                            WIDTH,
                            button_indicator_rect(event.index),
                        ),
                    );
                    report_display_error(
                        &mut uart,
                        display.update_region(
                            &timer,
                            framebuffer(),
                            WIDTH,
                            Rect {
                                x: 4,
                                y: 171,
                                width: 180,
                                height: 11,
                            },
                        ),
                    );
                }
            }
            let uptime_second = now_ms / 1_000;
            if uptime_second != last_uptime_second {
                last_uptime_second = uptime_second;
                draw_uptime(uptime_second);
                if display_ok {
                    report_display_error(
                        &mut uart,
                        display.update_region(
                            &timer,
                            framebuffer(),
                            WIDTH,
                            Rect {
                                x: 4,
                                y: 39,
                                width: 190,
                                height: 10,
                            },
                        ),
                    );
                }
            }
        }
        core::hint::spin_loop();
    }
}

fn draw_dashboard(current_el: u64, spi_hz: u32, transfer_us: u64) {
    let mut surface = framebuffer_surface();
    surface.clear(Rgb565::BLACK);
    surface.text(4, 4, "PIE8 HARDWARE DIAGNOSTICS", Rgb565::CYAN);
    surface.horizontal_line(4, 13, 188, Rgb565::CYAN);
    let mut text = TextBuffer::new();
    write!(
        text,
        "VERSION {} BUILD {}",
        env!("CARGO_PKG_VERSION"),
        env!("PIE8_BUILD_ID")
    )
    .ok();
    surface.text(4, 18, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(text, "EXCEPTION EL{}", exception_level(current_el)).ok();
    surface.text(4, 29, text.as_str(), Rgb565::WHITE);
    surface.text(4, 40, "UPTIME 0 S", Rgb565::WHITE);
    text.clear();
    write!(text, "SPI0 MODE0 CE0 {} KHZ", spi_hz / 1_000).ok();
    surface.text(4, 51, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(text, "FULL XFER {} US", transfer_us).ok();
    surface.text(4, 62, text.as_str(), Rgb565::WHITE);
    surface.text(4, 74, "BUTTONS ACTIVE LOW", Rgb565::YELLOW);
    for index in 0..8 {
        draw_button_on(&mut surface, index, false);
    }
    surface.text(4, 173, "TOTAL PRESSES 0", Rgb565::WHITE);
    surface.text(4, 190, "ST7789 RGB565 320X240", Rgb565::GREEN);
}

fn button_rect(index: usize) -> Rect {
    Rect {
        x: 4 + (index % 4) as i32 * 78,
        y: 88 + (index / 4) as i32 * 36,
        width: 72,
        height: 28,
    }
}

fn draw_button(index: usize, pressed: bool) {
    let mut surface = framebuffer_surface();
    surface.fill_rect(
        button_indicator_rect(index),
        if pressed {
            Rgb565::GREEN
        } else {
            Rgb565::new(48, 48, 48)
        },
    );
}

fn draw_button_on(surface: &mut Surface<'_>, index: usize, pressed: bool) {
    let rect = button_rect(index);
    surface.fill_rect(rect, Rgb565::new(24, 24, 24));
    surface.fill_rect(
        button_indicator_rect(index),
        if pressed {
            Rgb565::GREEN
        } else {
            Rgb565::new(48, 48, 48)
        },
    );
    let mut text = TextBuffer::new();
    write!(text, "BUTTON {}", index + 1).ok();
    surface.text(rect.x + 17, rect.y + 10, text.as_str(), Rgb565::WHITE);
}

fn button_indicator_rect(index: usize) -> Rect {
    let button = button_rect(index);
    Rect {
        x: button.x + 3,
        y: button.y + 3,
        width: 10,
        height: 22,
    }
}

fn draw_uptime(seconds: u64) {
    let mut surface = framebuffer_surface();
    surface.fill_rect(
        Rect {
            x: 4,
            y: 39,
            width: 190,
            height: 10,
        },
        Rgb565::BLACK,
    );
    let mut text = TextBuffer::new();
    write!(text, "UPTIME {seconds} S").ok();
    surface.text(4, 40, text.as_str(), Rgb565::WHITE);
}

fn draw_press_count(count: u64) {
    let mut surface = framebuffer_surface();
    surface.fill_rect(
        Rect {
            x: 4,
            y: 171,
            width: 180,
            height: 11,
        },
        Rgb565::BLACK,
    );
    let mut text = TextBuffer::new();
    write!(text, "TOTAL PRESSES {count}").ok();
    surface.text(4, 173, text.as_str(), Rgb565::WHITE);
}

fn report_display_error(uart: &mut MiniUart, result: Result<(), pie8_rpi::spi::SpiError>) {
    if let Err(error) = result {
        writeln!(uart, "display transfer failed: {error:?}").ok();
    }
}

fn framebuffer_surface() -> Surface<'static> {
    Surface::new(framebuffer_mut(), WIDTH as u32, HEIGHT as u32)
}

fn framebuffer_mut() -> &'static mut [u16] {
    // SAFETY: See Framebuffer's Sync invariant. The returned borrow is used only
    // transiently by the single boot core and never retained across another call.
    unsafe { &mut *FRAMEBUFFER.0.get() }
}

fn framebuffer() -> &'static [u16] {
    // SAFETY: Single-core access; mutable drawing borrows have ended before reads.
    unsafe { &*FRAMEBUFFER.0.get() }
}

struct TextBuffer {
    bytes: [u8; 96],
    length: usize,
}

impl TextBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; 96],
            length: 0,
        }
    }
    fn clear(&mut self) {
        self.length = 0;
    }
    fn as_str(&self) -> &str {
        // SAFETY: fmt::Write only accepts valid UTF-8 strings and copies their bytes.
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.length]) }
    }
}

impl fmt::Write for TextBuffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let remaining = self.bytes.len() - self.length;
        if text.len() > remaining {
            return Err(fmt::Error);
        }
        self.bytes[self.length..self.length + text.len()].copy_from_slice(text.as_bytes());
        self.length += text.len();
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut uart = MiniUart::new();
    writeln!(uart, "PANIC: {info}").ok();
    cpu::data_sync_barrier();
    cpu::wait_forever()
}

#[unsafe(no_mangle)]
extern "C" fn exception_handler(vector: u64) -> ! {
    let (esr, elr, far, spsr) = exception_registers();
    let mut uart = MiniUart::new();
    writeln!(uart, "FATAL EXCEPTION vector={vector}").ok();
    writeln!(uart, "ESR_EL1:  0x{esr:016x}").ok();
    writeln!(uart, "ELR_EL1:  0x{elr:016x}").ok();
    writeln!(uart, "FAR_EL1:  0x{far:016x}").ok();
    writeln!(uart, "SPSR_EL1: 0x{spsr:016x}").ok();
    cpu::data_sync_barrier();
    cpu::wait_forever()
}

fn exception_registers() -> (u64, u64, u64, u64) {
    let esr: u64;
    let elr: u64;
    let far: u64;
    let spsr: u64;
    // SAFETY: These EL1 fault registers are readable in an EL1 exception handler.
    unsafe {
        core::arch::asm!(
            "mrs {esr}, ESR_EL1", "mrs {elr}, ELR_EL1", "mrs {far}, FAR_EL1", "mrs {spsr}, SPSR_EL1",
            esr = out(reg) esr, elr = out(reg) elr, far = out(reg) far, spsr = out(reg) spsr,
            options(nomem, nostack)
        );
    }
    (esr, elr, far, spsr)
}

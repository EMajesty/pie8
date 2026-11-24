#![no_main]
#![no_std]

mod benchmark;
mod board;
mod display;

use core::{
    cell::UnsafeCell,
    fmt::{self, Write},
    panic::PanicInfo,
};

use benchmark::CpuBenchmarks;
use display::{ColorOrder, St7789};
use pie8_core::{
    RUST_READY_MESSAGE,
    audio::{AudioProcessor, SignalGenerator, StereoFrame, Waveform},
    buttons::{ButtonEventKind, Buttons},
    core_id, exception_level,
    fat32::DirectoryEntry,
    fat32::Fat32,
    graphics::{Rect, Rgb565, Surface},
    partition::{Partition, first_fat32},
    sample::{SampleId, SampleLibrary},
    wav::Wave,
};
use pie8_rpi::{
    cpu,
    gpio::{Function, Pull},
    mailbox, memory,
    mini_uart::MiniUart,
    sdhost::{InitStage, SdHost},
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
const SPI_DIVIDERS: [u16; 4] = [8, 10, 16, 32];

#[repr(align(64))]
struct Framebuffer(UnsafeCell<[u16; WIDTH * HEIGHT]>);
// SAFETY: Startup parks all cores except core 0, which serializes framebuffer access.
unsafe impl Sync for Framebuffer {}
#[unsafe(link_section = ".framebuffer")]
static FRAMEBUFFER: Framebuffer = Framebuffer(UnsafeCell::new([0; WIDTH * HEIGHT]));

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("vectors.S"));

#[derive(Clone, Copy, Eq, PartialEq)]
enum Page {
    Buttons,
    Colors,
    Performance,
    Audio,
    Storage,
    Samples,
}

impl Page {
    fn next(self) -> Self {
        match self {
            Self::Buttons => Self::Colors,
            Self::Colors => Self::Performance,
            Self::Performance => Self::Audio,
            Self::Audio => Self::Storage,
            Self::Storage => Self::Samples,
            Self::Samples => Self::Buttons,
        }
    }
    fn previous(self) -> Self {
        match self {
            Self::Buttons => Self::Samples,
            Self::Colors => Self::Buttons,
            Self::Performance => Self::Colors,
            Self::Audio => Self::Performance,
            Self::Storage => Self::Audio,
            Self::Samples => Self::Storage,
        }
    }
}

struct AppState<'a> {
    page: Page,
    total_presses: u64,
    inverted: bool,
    color_order: ColorOrder,
    spi_index: usize,
    uncached: CpuBenchmarks,
    cached: CpuBenchmarks,
    render_us: u64,
    spi_full_us: [u64; 4],
    spi_partial_us: [u64; 4],
    generator: SignalGenerator,
    generator_active: bool,
    audio_blocks: u64,
    audio_late_blocks: u64,
    detected_ram_bytes: Option<u64>,
    mapped_ram_bytes: u64,
    board_revision: Option<u32>,
    sd_initialized: bool,
    fat_mounted: bool,
    partition: Option<Partition>,
    sd_read_kib_s: u64,
    sd_stage: InitStage,
    sd_last_command: u8,
    sd_last_response: u32,
    sd_status: u32,
    sd_blocks: u64,
    sample_scratch: &'a mut [u8],
    samples: SampleLibrary<'a, 8>,
    browser_entries: [Option<DirectoryEntry>; 8],
    browser_count: usize,
    browser_selection: usize,
    browser_directory: Option<DirectoryEntry>,
    loaded_sample: Option<SampleId>,
    sample_load_us: u64,
    sample_error: Option<&'static str>,
    waveform: [u8; 64],
}

#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    board::assert_wiring();
    let current_el = cpu::current_el();
    let timer = Timer::new();
    let uncached = benchmark::run_cpu(&timer);
    let detected_memory = mailbox::arm_memory(&timer, 100_000).ok();
    let board_revision = mailbox::board_revision(&timer, 100_000).ok();
    let memory_map = memory::enable_mmu_and_caches(detected_memory.map(|m| u64::from(m.size)));
    let sample_region = memory::sample_region(memory_map);
    let sample_memory: &'static mut [u8] = sample_region.map_or(&mut [], |region| {
        // SAFETY: OwnedRegion is disjoint from the linked image and exclusively
        // transferred to this arena for the rest of the firmware lifetime.
        unsafe { core::slice::from_raw_parts_mut(region.start as *mut u8, region.length) }
    });
    let scratch_length = sample_memory.len().min(8 * 1024 * 1024);
    let (sample_scratch, sample_storage) = sample_memory.split_at_mut(scratch_length);
    let cached = benchmark::run_cpu(&timer);
    let mut uart = MiniUart::new();
    writeln!(
        uart,
        "PIE8 runtime/audio diagnostics {} {}",
        env!("CARGO_PKG_VERSION"),
        env!("PIE8_BUILD_ID")
    )
    .ok();
    writeln!(
        uart,
        "EL{} core {} counter {} Hz",
        exception_level(current_el),
        core_id(cpu::mpidr()),
        timer.frequency_hz()
    )
    .ok();
    writeln!(
        uart,
        "MMU+caches enabled; mapped RAM {} MiB; MMIO {:#x}..{:#x}",
        memory_map.ram_mapped_bytes / 1_048_576,
        memory_map.peripheral_start,
        memory_map.peripheral_end
    )
    .ok();
    writeln!(uart, "{RUST_READY_MESSAGE}").ok();

    for pin in board::BUTTONS {
        pin.set_function(Function::Input);
        pin.set_pull(Pull::Up, &timer);
    }
    let spi = Spi0::new(&timer, board::SPI_CLOCK_DIVIDER, board::SPI_TIMEOUT_US);
    let mut display = St7789::new(
        spi,
        board::LCD_DC,
        board::LCD_RESET,
        board::LCD_BACKLIGHT,
        board::DISPLAY_CONFIG,
    );
    let display_ok = display.init(&timer).is_ok();
    writeln!(
        uart,
        "display init: {} inversion default OFF",
        if display_ok { "sent" } else { "failed" }
    )
    .ok();

    let sd = SdHost::new(timer);
    let (sd_initialized, partition, fat_mounted, sd_read_kib_s) = initialize_storage(&sd, &timer);
    let sd_diag = sd.diagnostics();
    writeln!(uart, "board revision {board_revision:?}; SD init={sd_initialized} stage={:?} cmd={} rsp={:#010x} hsts={:#010x}", sd_diag.stage, sd_diag.last_command, sd_diag.last_response, sd_diag.status).ok();

    let render_us = benchmark::render(&timer, framebuffer_mut(), WIDTH as u32, HEIGHT as u32);
    let mut state = AppState {
        page: Page::Colors,
        total_presses: 0,
        inverted: false,
        color_order: ColorOrder::Rgb,
        spi_index: 0,
        uncached,
        cached,
        render_us,
        spi_full_us: [0; 4],
        spi_partial_us: [0; 4],
        generator: SignalGenerator::new(),
        generator_active: false,
        audio_blocks: 0,
        audio_late_blocks: 0,
        detected_ram_bytes: memory_map.detected_ram_bytes,
        mapped_ram_bytes: memory_map.ram_mapped_bytes,
        board_revision,
        sd_initialized,
        fat_mounted,
        partition,
        sd_read_kib_s,
        sd_stage: sd_diag.stage,
        sd_last_command: sd_diag.last_command,
        sd_last_response: sd_diag.last_response,
        sd_status: sd_diag.status,
        sd_blocks: sd_diag.blocks,
        sample_scratch,
        samples: SampleLibrary::new(sample_storage),
        browser_entries: [None; 8],
        browser_count: 0,
        browser_selection: 0,
        browser_directory: None,
        loaded_sample: None,
        sample_load_us: 0,
        sample_error: None,
        waveform: [0; 64],
    };
    scan_samples(&mut state, &sd, None);
    draw_page(&state, current_el);
    if display_ok {
        log_display(
            &mut uart,
            display.update_region(&timer, framebuffer(), WIDTH, FULL_SCREEN),
        );
    }

    let mut buttons = Buttons::<8>::default();
    let mut next_poll_ms = timer.millis();
    let mut next_audio_us = timer.micros();
    let mut next_status_ms = timer.millis().wrapping_add(1_000);
    let mut audio_block = [StereoFrame::default(); 128];

    loop {
        let now_us = timer.micros();
        if state.generator_active && now_us.wrapping_sub(next_audio_us) as i64 >= 0 {
            let lateness = now_us.wrapping_sub(next_audio_us);
            if lateness > 2_667 {
                state.audio_late_blocks += 1;
            }
            state.generator.process(&mut audio_block);
            state.audio_blocks += 1;
            // Do not spin through a backlog after a blocking display transfer.
            next_audio_us = now_us.wrapping_add(2_667);
        }

        let now_ms = timer.millis();
        if now_ms.wrapping_sub(next_poll_ms) as i64 >= 0 {
            next_poll_ms = now_ms.wrapping_add(1);
            let samples = board::BUTTONS.map(|pin| !pin.level());
            for event in buttons.update(samples, now_ms).into_iter().flatten() {
                if event.kind == ButtonEventKind::Pressed {
                    state.total_presses += 1;
                    writeln!(uart, "button {} pressed", event.index + 1).ok();
                    let redraw = handle_press(
                        event.index,
                        &mut state,
                        &timer,
                        &mut display,
                        display_ok,
                        &mut uart,
                        current_el,
                        &sd,
                    );
                    if redraw && display_ok {
                        log_display(
                            &mut uart,
                            display.update_region(&timer, framebuffer(), WIDTH, FULL_SCREEN),
                        );
                    }
                } else {
                    writeln!(
                        uart,
                        "button {} released held={}ms",
                        event.index + 1,
                        event.duration_ms
                    )
                    .ok();
                }
                if state.page == Page::Buttons {
                    draw_button_indicator(event.index, buttons.is_pressed(event.index));
                    if display_ok {
                        log_display(
                            &mut uart,
                            display.update_region(
                                &timer,
                                framebuffer(),
                                WIDTH,
                                button_indicator_rect(event.index),
                            ),
                        );
                    }
                }
            }
        }
        if now_ms.wrapping_sub(next_status_ms) as i64 >= 0 {
            next_status_ms = now_ms.wrapping_add(1_000);
            if state.page == Page::Audio {
                draw_audio_stats(&state);
                if display_ok {
                    log_display(
                        &mut uart,
                        display.update_region(
                            &timer,
                            framebuffer(),
                            WIDTH,
                            Rect {
                                x: 4,
                                y: 151,
                                width: 310,
                                height: 38,
                            },
                        ),
                    );
                }
            }
        }
        core::hint::spin_loop();
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_press(
    index: usize,
    state: &mut AppState<'_>,
    timer: &Timer,
    display: &mut St7789,
    display_ok: bool,
    uart: &mut MiniUart,
    current_el: u64,
    sd: &SdHost,
) -> bool {
    if index == 6 {
        state.generator_active = false;
        state.page = state.page.previous();
        draw_page(state, current_el);
        return true;
    }
    if index == 7 {
        state.generator_active = false;
        state.page = state.page.next();
        draw_page(state, current_el);
        return true;
    }
    match state.page {
        Page::Buttons => {
            draw_page(state, current_el);
            true
        }
        Page::Colors => {
            match index {
                0 => {
                    state.inverted = !state.inverted;
                    if display_ok {
                        log_display(uart, display.set_inverted(timer, state.inverted));
                    }
                }
                1 => {
                    state.color_order = match state.color_order {
                        ColorOrder::Rgb => ColorOrder::Bgr,
                        ColorOrder::Bgr => ColorOrder::Rgb,
                    };
                    if display_ok {
                        log_display(uart, display.set_color_order(timer, state.color_order));
                    }
                }
                2 => {
                    state.spi_index = (state.spi_index + 1) % SPI_DIVIDERS.len();
                    display.set_spi_divider(SPI_DIVIDERS[state.spi_index]);
                }
                3 => {
                    state.spi_index = 3;
                    display.set_spi_divider(SPI_DIVIDERS[3]);
                }
                _ => {}
            }
            draw_page(state, current_el);
            true
        }
        Page::Performance => {
            if index == 0 && display_ok {
                run_spi_benchmarks(state, timer, display, uart);
            }
            if index == 1 {
                state.cached = benchmark::run_cpu(timer);
                state.render_us =
                    benchmark::render(timer, framebuffer_mut(), WIDTH as u32, HEIGHT as u32);
            }
            draw_page(state, current_el);
            true
        }
        Page::Audio => {
            match index {
                0 => state.generator_active = !state.generator_active,
                1 => state
                    .generator
                    .set_waveform(next_waveform(state.generator.waveform())),
                2 => state
                    .generator
                    .set_frequency_hz(state.generator.frequency_hz() - 10.0),
                3 => state
                    .generator
                    .set_frequency_hz(state.generator.frequency_hz() + 10.0),
                4 => state.generator.set_gain(state.generator.gain() - 0.05),
                5 => state.generator.set_gain(state.generator.gain() + 0.05),
                _ => {}
            }
            draw_page(state, current_el);
            true
        }
        Page::Storage => {
            if index == 0 {
                let (initialized, partition, mounted, read_kib_s) = initialize_storage(sd, timer);
                state.sd_initialized = initialized;
                state.partition = partition;
                state.fat_mounted = mounted;
                state.sd_read_kib_s = read_kib_s;
                let d = sd.diagnostics();
                state.sd_stage = d.stage;
                state.sd_last_command = d.last_command;
                state.sd_last_response = d.last_response;
                state.sd_status = d.status;
                state.sd_blocks = d.blocks;
                if mounted {
                    scan_samples(state, sd, None);
                } else {
                    state.browser_entries.fill(None);
                    state.browser_count = 0;
                }
                writeln!(uart, "SD retry init={initialized} stage={:?} cmd={} rsp={:#010x} hsts={:#010x} error={:?}", d.stage, d.last_command, d.last_response, d.status, d.last_error).ok();
                draw_page(state, current_el);
                true
            } else {
                false
            }
        }
        Page::Samples => {
            match index {
                0 => load_selected_sample(state, sd, timer),
                1 => {
                    state.samples.reset();
                    state.loaded_sample = None;
                    state.sample_error = None;
                    state.waveform.fill(0);
                }
                2 if state.browser_count != 0 => {
                    state.browser_selection = state.browser_selection.saturating_sub(1);
                }
                3 if state.browser_count != 0 => {
                    state.browser_selection = (state.browser_selection + 1) % state.browser_count;
                }
                4 => {
                    if let Some(entry) = state.browser_entries[state.browser_selection] {
                        if entry.is_directory() {
                            scan_samples(state, sd, Some(entry));
                        }
                    }
                }
                5 => scan_samples(state, sd, None),
                _ => {}
            }
            draw_page(state, current_el);
            true
        }
    }
}

fn scan_samples(state: &mut AppState<'_>, sd: &SdHost, directory: Option<DirectoryEntry>) {
    state.sample_error = None;
    state.browser_entries.fill(None);
    state.browser_count = 0;
    state.browser_selection = 0;
    state.browser_directory = directory;
    let Some(partition) = state.partition else {
        state.sample_error = Some("NO FAT32 PARTITION");
        return;
    };
    let Ok(fs) = Fat32::mount(sd, partition) else {
        state.sample_error = Some("FAT32 MOUNT FAILED");
        return;
    };
    let root_samples = if directory.is_none() {
        let Ok(samples) = fs.find_path("samples") else {
            state.sample_error = Some("/SAMPLES NOT FOUND");
            return;
        };
        Some(samples)
    } else {
        directory
    };
    state.browser_directory = root_samples;
    if fs
        .enumerate(root_samples.as_ref(), |entry| {
            if state.browser_count == state.browser_entries.len() {
                return false;
            }
            if entry.is_directory() || entry.is_wav() {
                state.browser_entries[state.browser_count] = Some(*entry);
                state.browser_count += 1;
            }
            true
        })
        .is_err()
    {
        state.sample_error = Some("DIRECTORY READ FAILED");
    }
}

fn load_selected_sample(state: &mut AppState<'_>, sd: &SdHost, timer: &Timer) {
    state.sample_error = None;
    let Some(entry) = state
        .browser_entries
        .get(state.browser_selection)
        .copied()
        .flatten()
    else {
        state.sample_error = Some("NO FILE");
        return;
    };
    if entry.is_directory() {
        state.sample_error = Some("SELECT A WAV");
        return;
    }
    let Some(partition) = state.partition else {
        state.sample_error = Some("NO FAT32");
        return;
    };
    let Ok(fs) = Fat32::mount(sd, partition) else {
        state.sample_error = Some("MOUNT FAILED");
        return;
    };
    if entry.size as usize > state.sample_scratch.len() {
        state.sample_error = Some("FILE TOO LARGE");
        return;
    }
    let start = timer.micros();
    let bytes = match fs.read_file(&entry, state.sample_scratch) {
        Ok(bytes) => bytes,
        Err(_) => {
            state.sample_error = Some("READ FAILED");
            return;
        }
    };
    let wave = match Wave::parse(&state.sample_scratch[..bytes]) {
        Ok(wave) => wave,
        Err(_) => {
            state.sample_error = Some("UNSUPPORTED WAV");
            return;
        }
    };
    for (index, height) in state.waveform.iter_mut().enumerate() {
        let frame = (u64::from(wave.frame_count) * index as u64 / 64) as u32;
        *height = wave
            .frame(frame)
            .map_or(0, |f| (f.left.abs().max(f.right.abs()) * 36.0) as u8);
    }
    state.samples.reset();
    match state
        .samples
        .load(entry.name(), &state.sample_scratch[..bytes])
    {
        Ok(id) => {
            state.loaded_sample = Some(id);
            state.sample_load_us = timer.micros().wrapping_sub(start);
        }
        Err(_) => state.sample_error = Some("ARENA EXHAUSTED"),
    }
}

fn initialize_storage(sd: &SdHost, timer: &Timer) -> (bool, Option<Partition>, bool, u64) {
    if sd.initialize().is_err() {
        return (false, None, false, 0);
    }
    let partition = first_fat32(sd).ok();
    let mounted = partition.and_then(|p| Fat32::mount(sd, p).ok()).is_some();
    let mut sector = [0u8; 512];
    let start = timer.micros();
    let mut read = 0u64;
    for lba in 0..128u64 {
        if pie8_core::block::ReadBlockDevice::read_blocks(sd, lba, &mut sector).is_err() {
            break;
        }
        read += 512;
    }
    let elapsed = timer.micros().wrapping_sub(start).max(1);
    (
        true,
        partition,
        mounted,
        read.saturating_mul(1_000_000) / elapsed / 1024,
    )
}

fn run_spi_benchmarks(
    state: &mut AppState<'_>,
    timer: &Timer,
    display: &mut St7789,
    uart: &mut MiniUart,
) {
    draw_color_grid();
    for (index, divider) in SPI_DIVIDERS.into_iter().enumerate() {
        display.set_spi_divider(divider);
        let start = timer.micros();
        if display
            .update_region(timer, framebuffer(), WIDTH, FULL_SCREEN)
            .is_ok()
        {
            state.spi_full_us[index] = timer.micros().wrapping_sub(start);
        }
        let start = timer.micros();
        if display
            .update_region(
                timer,
                framebuffer(),
                WIDTH,
                Rect {
                    x: 0,
                    y: 0,
                    width: 64,
                    height: 64,
                },
            )
            .is_ok()
        {
            state.spi_partial_us[index] = timer.micros().wrapping_sub(start);
        }
        writeln!(
            uart,
            "SPI divider {divider}: full={}us partial={}us",
            state.spi_full_us[index], state.spi_partial_us[index]
        )
        .ok();
    }
    state.spi_index = 0;
    display.set_spi_divider(SPI_DIVIDERS[0]);
}

fn draw_page(state: &AppState<'_>, current_el: u64) {
    match state.page {
        Page::Buttons => draw_buttons_page(state, current_el),
        Page::Colors => draw_colors_page(state),
        Page::Performance => draw_performance_page(state),
        Page::Audio => draw_audio_page(state),
        Page::Storage => draw_storage_page(state),
        Page::Samples => draw_samples_page(state),
    }
}

fn page_header(surface: &mut Surface<'_>, title: &str) {
    surface.clear(Rgb565::BLACK);
    surface.text(4, 4, title, Rgb565::CYAN);
    surface.horizontal_line(4, 14, 312, Rgb565::CYAN);
    surface.text(4, 228, "B7 PREV  B8 NEXT", Rgb565::WHITE);
}

fn draw_buttons_page(state: &AppState<'_>, current_el: u64) {
    let mut surface = framebuffer_surface();
    page_header(&mut surface, "BUTTON TEST");
    let mut text = TextBuffer::new();
    write!(
        text,
        "EL{} TOTAL PRESSES {}",
        exception_level(current_el),
        state.total_presses
    )
    .ok();
    surface.text(4, 23, text.as_str(), Rgb565::WHITE);
    for index in 0..8 {
        draw_button_on(&mut surface, index, false);
    }
}

fn draw_colors_page(state: &AppState<'_>) {
    draw_color_grid();
    let mut surface = framebuffer_surface();
    surface.fill_rect(
        Rect {
            x: 0,
            y: 0,
            width: 320,
            height: 32,
        },
        Rgb565::BLACK,
    );
    surface.text(4, 3, "DISPLAY COLOUR CONFIG", Rgb565::WHITE);
    let mut text = TextBuffer::new();
    write!(
        text,
        "INV {} ORDER {} SPI {}KHZ",
        if state.inverted { "ON" } else { "OFF" },
        if matches!(state.color_order, ColorOrder::Rgb) {
            "RGB"
        } else {
            "BGR"
        },
        board::CORE_CLOCK_HZ / 1_000 / SPI_DIVIDERS[state.spi_index] as u32
    )
    .ok();
    surface.text(4, 15, text.as_str(), Rgb565::WHITE);
    surface.fill_rect(
        Rect {
            x: 0,
            y: 216,
            width: 320,
            height: 24,
        },
        Rgb565::BLACK,
    );
    surface.text(4, 217, "B1 INV B2 RGB B3 SPEED B4 SAFE", Rgb565::WHITE);
    surface.text(4, 229, "B7 PREV B8 NEXT", Rgb565::WHITE);
}

fn draw_color_grid() {
    let colors = [
        ("RED", Rgb565::RED, Rgb565::WHITE),
        ("GREEN", Rgb565::GREEN, Rgb565::BLACK),
        ("BLUE", Rgb565::BLUE, Rgb565::WHITE),
        ("CYAN", Rgb565::CYAN, Rgb565::BLACK),
        ("MAGENTA", Rgb565::MAGENTA, Rgb565::WHITE),
        ("YELLOW", Rgb565::YELLOW, Rgb565::BLACK),
        ("WHITE", Rgb565::WHITE, Rgb565::BLACK),
        ("BLACK", Rgb565::BLACK, Rgb565::WHITE),
    ];
    let mut surface = framebuffer_surface();
    for (index, (label, color, text)) in colors.into_iter().enumerate() {
        let x = (index % 4) as i32 * 80;
        let y = 36 + (index / 4) as i32 * 88;
        surface.fill_rect(
            Rect {
                x,
                y,
                width: 80,
                height: 88,
            },
            color,
        );
        surface.text(x + 5, y + 39, label, text);
    }
}

fn draw_performance_page(state: &AppState<'_>) {
    let mut surface = framebuffer_surface();
    page_header(&mut surface, "PERFORMANCE BENCHMARKS");
    let mut y = 23;
    let mut text = TextBuffer::new();
    for (name, before, after) in [
        (
            "INT KOPS",
            state.uncached.integer_ops_s / 1000,
            state.cached.integer_ops_s / 1000,
        ),
        (
            "FLOAT KOPS",
            state.uncached.float_ops_s / 1000,
            state.cached.float_ops_s / 1000,
        ),
        (
            "COPY KB/S",
            state.uncached.copy_bytes_s / 1024,
            state.cached.copy_bytes_s / 1024,
        ),
    ] {
        text.clear();
        write!(text, "{} OFF {} ON {}", name, before, after).ok();
        surface.text(4, y, text.as_str(), Rgb565::WHITE);
        y += 12;
    }
    text.clear();
    write!(text, "RENDER 200 RECTS {} US", state.render_us).ok();
    surface.text(4, y, text.as_str(), Rgb565::WHITE);
    y += 18;
    for (index, divider) in SPI_DIVIDERS.into_iter().enumerate() {
        text.clear();
        write!(
            text,
            "SPI {}KHZ FULL {} PART {} US",
            board::CORE_CLOCK_HZ / 1_000 / divider as u32,
            state.spi_full_us[index],
            state.spi_partial_us[index]
        )
        .ok();
        surface.text(4, y, text.as_str(), Rgb565::WHITE);
        y += 12;
    }
    surface.text(4, 190, "B1 RUN SPI B2 RERUN CPU", Rgb565::YELLOW);
    surface.text(4, 203, "SPI RESTORES DEFAULT 31250 KHZ", Rgb565::GREEN);
}

fn draw_audio_page(state: &AppState<'_>) {
    {
        let mut surface = framebuffer_surface();
        page_header(&mut surface, "AUDIO TESTING");
        surface.text(4, 24, "PWM DMA BACKEND BLOCKED", Rgb565::YELLOW);
        surface.text(4, 36, "NO PHYSICAL AUDIO OUTPUT", Rgb565::YELLOW);
        let mut text = TextBuffer::new();
        write!(
            text,
            "GENERATOR {}",
            if state.generator_active {
                "RUNNING"
            } else {
                "STOPPED"
            }
        )
        .ok();
        surface.text(4, 58, text.as_str(), Rgb565::WHITE);
        text.clear();
        write!(text, "WAVE {:?}", state.generator.waveform()).ok();
        surface.text(4, 72, text.as_str(), Rgb565::WHITE);
        text.clear();
        write!(
            text,
            "FREQ {} HZ GAIN {} PCT",
            state.generator.frequency_hz() as u32,
            (state.generator.gain() * 100.0) as u32
        )
        .ok();
        surface.text(4, 86, text.as_str(), Rgb565::WHITE);
        surface.text(4, 108, "48KHZ STEREO F32 BLOCK 128", Rgb565::GREEN);
        surface.text(4, 122, "B1 START B2 WAVE B3/4 FREQ", Rgb565::WHITE);
        surface.text(4, 135, "B5/6 VOLUME", Rgb565::WHITE);
    }
    draw_audio_stats(state);
}

fn draw_audio_stats(state: &AppState<'_>) {
    let mut surface = framebuffer_surface();
    surface.fill_rect(
        Rect {
            x: 4,
            y: 151,
            width: 310,
            height: 38,
        },
        Rgb565::BLACK,
    );
    let mut text = TextBuffer::new();
    write!(text, "GENERATED BLOCKS {}", state.audio_blocks).ok();
    surface.text(4, 153, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(text, "LATE BLOCKS {} UNDERRUN N/A", state.audio_late_blocks).ok();
    surface.text(4, 167, text.as_str(), Rgb565::WHITE);
}

fn draw_storage_page(state: &AppState<'_>) {
    let mut surface = framebuffer_surface();
    page_header(&mut surface, "STORAGE DIAGNOSTIC");
    let mut text = TextBuffer::new();
    write!(
        text,
        "ARM RAM {} MIB",
        state.detected_ram_bytes.map_or(0, |n| n / 1_048_576)
    )
    .ok();
    surface.text(4, 26, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(text, "MAPPED {} MIB", state.mapped_ram_bytes / 1_048_576).ok();
    surface.text(4, 40, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(
        text,
        "BOARD REV {:#010X}",
        state.board_revision.unwrap_or(0)
    )
    .ok();
    surface.text(4, 54, text.as_str(), Rgb565::WHITE);
    surface.text(4, 68, "SDHOST 3F202000 GPIO48-53 ALT0", Rgb565::CYAN);
    surface.text(
        4,
        82,
        if state.sd_initialized {
            "CARD: READY"
        } else {
            "CARD: FAILED (B1 RETRY)"
        },
        if state.sd_initialized {
            Rgb565::GREEN
        } else {
            Rgb565::YELLOW
        },
    );
    text.clear();
    write!(
        text,
        "STAGE {:?} CAP {} MIB",
        state.sd_stage,
        state.sd_blocks / 2048
    )
    .ok();
    surface.text(4, 96, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(
        text,
        "FAT32 {} PART LBA {}",
        if state.fat_mounted {
            "MOUNTED"
        } else {
            "FAILED"
        },
        state.partition.map_or(0, |p| p.first_lba)
    )
    .ok();
    surface.text(4, 110, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(
        text,
        "CMD{} RSP {:08X}",
        state.sd_last_command, state.sd_last_response
    )
    .ok();
    surface.text(4, 124, text.as_str(), Rgb565::WHITE);
    text.clear();
    write!(
        text,
        "HSTS {:08X} READ {} KIB/S",
        state.sd_status, state.sd_read_kib_s
    )
    .ok();
    surface.text(4, 138, text.as_str(), Rgb565::WHITE);
    surface.text(4, 162, "B1 RETRY SD INITIALIZATION", Rgb565::YELLOW);
    surface.text(4, 178, "READ-ONLY: NO WRITE/ERASE COMMANDS", Rgb565::GREEN);
}

fn draw_samples_page(state: &AppState<'_>) {
    let mut surface = framebuffer_surface();
    page_header(&mut surface, "SAMPLE BROWSER");
    let mut text = TextBuffer::new();
    write!(
        text,
        "FILES {} ARENA {} MIB",
        state.browser_count,
        state.samples.capacity_bytes() / 1_048_576
    )
    .ok();
    surface.text(4, 24, text.as_str(), Rgb565::WHITE);
    if let Some(entry) = state
        .browser_entries
        .get(state.browser_selection)
        .copied()
        .flatten()
    {
        text.clear();
        write!(
            text,
            "SELECT {}{}",
            if entry.is_directory() { "DIR " } else { "" },
            entry.name()
        )
        .ok();
        surface.text(4, 40, text.as_str(), Rgb565::CYAN);
    } else {
        surface.text(
            4,
            40,
            state.sample_error.unwrap_or("NO WAV/DIRECTORY ENTRIES"),
            Rgb565::YELLOW,
        );
    }
    if let Some(id) = state.loaded_sample {
        if let Some(sample) = state.samples.get(id) {
            text.clear();
            write!(
                text,
                "{}BIT {}CH {}HZ",
                sample.bits_per_sample, sample.channels, sample.sample_rate
            )
            .ok();
            surface.text(4, 58, text.as_str(), Rgb565::GREEN);
            text.clear();
            write!(
                text,
                "{}MS {}B LOAD {}US",
                sample.duration_ms, sample.memory_bytes, state.sample_load_us
            )
            .ok();
            surface.text(4, 72, text.as_str(), Rgb565::WHITE);
        }
    } else if state.browser_count != 0 {
        if let Some(error) = state.sample_error {
            surface.text(4, 58, error, Rgb565::YELLOW);
        }
    }
    surface.horizontal_line(8, 138, 304, Rgb565::new(48, 48, 48));
    for (x, height) in state.waveform.iter().copied().enumerate() {
        surface.vertical_line(
            8 + (x as i32 * 4),
            138 - i32::from(height),
            u32::from(height) * 2 + 1,
            Rgb565::GREEN,
        );
    }
    surface.text(4, 182, "B1 LOAD B2 RELEASE B3/4 SELECT", Rgb565::WHITE);
    surface.text(4, 196, "B5 ENTER DIR B6 /SAMPLES", Rgb565::WHITE);
}

fn button_rect(index: usize) -> Rect {
    Rect {
        x: 4 + (index % 4) as i32 * 78,
        y: 52 + (index / 4) as i32 * 52,
        width: 72,
        height: 40,
    }
}
fn button_indicator_rect(index: usize) -> Rect {
    let button = button_rect(index);
    Rect {
        x: button.x + 3,
        y: button.y + 3,
        width: 10,
        height: 34,
    }
}
fn draw_button_indicator(index: usize, pressed: bool) {
    framebuffer_surface().fill_rect(
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
    surface.text(rect.x + 17, rect.y + 16, text.as_str(), Rgb565::WHITE);
}

fn next_waveform(waveform: Waveform) -> Waveform {
    match waveform {
        Waveform::Silence => Waveform::Sine,
        Waveform::Sine => Waveform::Triangle,
        Waveform::Triangle => Waveform::Square,
        Waveform::Square => Waveform::Silence,
    }
}
fn log_display(uart: &mut MiniUart, result: Result<(), pie8_rpi::spi::SpiError>) {
    if let Err(error) = result {
        writeln!(uart, "display error: {error:?}").ok();
    }
}
fn framebuffer_surface() -> Surface<'static> {
    Surface::new(framebuffer_mut(), WIDTH as u32, HEIGHT as u32)
}
fn framebuffer_mut() -> &'static mut [u16] {
    // SAFETY: Only core 0 runs and all mutable framebuffer borrows are transient.
    unsafe { &mut *FRAMEBUFFER.0.get() }
}
fn framebuffer() -> &'static [u16] {
    // SAFETY: Drawing borrows end before the single core begins an SPI read.
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
        // SAFETY: fmt::Write copies only valid UTF-8 into this prefix.
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.length]) }
    }
}
impl fmt::Write for TextBuffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > self.bytes.len() - self.length {
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
    writeln!(
        uart,
        "ESR={esr:016x} ELR={elr:016x} FAR={far:016x} SPSR={spsr:016x}"
    )
    .ok();
    cpu::data_sync_barrier();
    cpu::wait_forever()
}
fn exception_registers() -> (u64, u64, u64, u64) {
    let esr: u64;
    let elr: u64;
    let far: u64;
    let spsr: u64;
    // SAFETY: These fault registers are readable while handling an EL1 exception.
    unsafe {
        core::arch::asm!("mrs {esr}, ESR_EL1", "mrs {elr}, ELR_EL1", "mrs {far}, FAR_EL1", "mrs {spsr}, SPSR_EL1", esr=out(reg) esr, elr=out(reg) elr, far=out(reg) far, spsr=out(reg) spsr, options(nomem, nostack));
    }
    (esr, elr, far, spsr)
}

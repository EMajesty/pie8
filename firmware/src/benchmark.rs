use core::{cell::UnsafeCell, hint::black_box};

use pie8_core::{
    benchmark::Rate,
    graphics::{Rect, Rgb565, Surface},
};
use pie8_rpi::timer::Timer;

const ITERATIONS: u64 = 200_000;
const SCRATCH_SIZE: usize = 16 * 1024;

struct Scratch(UnsafeCell<[u8; SCRATCH_SIZE]>);
// SAFETY: The single boot core has exclusive access during synchronous benchmarks.
unsafe impl Sync for Scratch {}
static SOURCE: Scratch = Scratch(UnsafeCell::new([0; SCRATCH_SIZE]));
static DESTINATION: Scratch = Scratch(UnsafeCell::new([0; SCRATCH_SIZE]));

#[derive(Clone, Copy, Default)]
pub struct CpuBenchmarks {
    pub integer_ops_s: u64,
    pub float_ops_s: u64,
    pub copy_bytes_s: u64,
}

pub fn run_cpu(timer: &Timer) -> CpuBenchmarks {
    let started = timer.micros();
    let mut integer = 0x1234_5678_9abc_def0_u64;
    for index in 0..ITERATIONS {
        integer = integer.rotate_left(7) ^ index.wrapping_mul(0x9e37_79b9);
        black_box(integer);
    }
    let integer_us = timer.micros().wrapping_sub(started).max(1);

    let started = timer.micros();
    let mut floating = 0.125_f32;
    for _ in 0..ITERATIONS {
        floating = floating * 1.000_001 + 0.000_003;
        if floating > 2.0 {
            floating -= 1.0;
        }
        black_box(floating);
    }
    let float_us = timer.micros().wrapping_sub(started).max(1);

    // SAFETY: Single-core benchmark owns both static arrays for this call.
    let (source, destination) = unsafe { (&mut *SOURCE.0.get(), &mut *DESTINATION.0.get()) };
    for (index, byte) in source.iter_mut().enumerate() {
        *byte = index as u8;
    }
    let copy_iterations = 64_u64;
    let started = timer.micros();
    for _ in 0..copy_iterations {
        destination.copy_from_slice(source);
        black_box(&destination[..]);
    }
    let copy_us = timer.micros().wrapping_sub(started).max(1);

    CpuBenchmarks {
        integer_ops_s: Rate {
            operations: ITERATIONS * 3,
            elapsed_us: integer_us,
        }
        .per_second(),
        float_ops_s: Rate {
            operations: ITERATIONS * 2,
            elapsed_us: float_us,
        }
        .per_second(),
        copy_bytes_s: Rate {
            operations: copy_iterations * SCRATCH_SIZE as u64,
            elapsed_us: copy_us,
        }
        .per_second(),
    }
}

pub fn render(timer: &Timer, pixels: &mut [u16], width: u32, height: u32) -> u64 {
    let started = timer.micros();
    let mut surface = Surface::new(pixels, width, height);
    for index in 0..200 {
        let x = (index * 17 % width as usize) as i32;
        let y = (index * 29 % height as usize) as i32;
        surface.fill_rect(
            Rect {
                x,
                y,
                width: 31,
                height: 19,
            },
            Rgb565::new(index as u8, (index * 3) as u8, (index * 7) as u8),
        );
    }
    timer.micros().wrapping_sub(started)
}

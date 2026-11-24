use core::{cell::UnsafeCell, hint::black_box};

use pie8_core::{
    audio::StereoFrame,
    benchmark::Rate,
    graphics::{Rect, Rgb565, Surface},
    sample::{SampleId, SampleLibrary},
    sampler::{EnvelopeParameters, LoopMode, Sampler, Voice, VoiceParameters},
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

#[derive(Clone, Copy, Default)]
pub struct DspResult {
    pub voices: u16,
    pub block_frames: u16,
    pub average_us: u64,
    pub maximum_us: u64,
    pub deadline_us: u64,
    pub misses: u32,
    pub utilization_tenths: u32,
    pub voice_memory_bytes: u32,
    pub checksum: f32,
}

pub fn sampler_benchmarks(
    timer: &Timer,
    samples: &SampleLibrary<'_, 8>,
    sample_id: SampleId,
) -> [DspResult; 15] {
    let mut results = [DspResult::default(); 15];
    let mut output = [StereoFrame::default(); 256];
    let mut result_index = 0;
    for voices in [8usize, 16, 32, 64, 128] {
        for block_frames in [64usize, 128, 256] {
            let mut sampler = Sampler::<128>::new();
            let parameters = VoiceParameters {
                end_frame: samples.get(sample_id).map_or(0, |s| s.frame_count),
                semitones: 3.25,
                gain: 0.25,
                pan: 0.2,
                loop_mode: LoopMode::Forward,
                envelope: EnvelopeParameters {
                    attack_ms: 1.0,
                    decay_ms: 10.0,
                    sustain: 0.8,
                    release_ms: 10.0,
                    declick_ms: 1.0,
                },
                ..VoiceParameters::default()
            };
            for _ in 0..voices {
                let _ = sampler.trigger(samples, sample_id, parameters);
            }
            let iterations = 128u64;
            let deadline_us = block_frames as u64 * 1_000_000 / 48_000;
            let mut total = 0u64;
            let mut maximum = 0u64;
            let mut misses = 0u32;
            let mut checksum = 0.0f32;
            for _ in 0..iterations {
                let start = timer.micros();
                sampler.process(samples, &mut output[..block_frames]);
                let elapsed = timer.micros().wrapping_sub(start);
                total += elapsed;
                maximum = maximum.max(elapsed);
                misses += u32::from(elapsed > deadline_us);
                checksum += output[block_frames - 1].left + output[block_frames / 2].right;
                black_box(checksum);
            }
            let average = total / iterations;
            results[result_index] = DspResult {
                voices: voices as u16,
                block_frames: block_frames as u16,
                average_us: average,
                maximum_us: maximum,
                deadline_us,
                misses,
                utilization_tenths: (average.saturating_mul(1_000) / deadline_us) as u32,
                voice_memory_bytes: (voices * core::mem::size_of::<Voice>()) as u32,
                checksum,
            };
            result_index += 1;
        }
    }
    results
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

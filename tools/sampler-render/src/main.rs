use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

use pie8_core::{
    audio::{SAMPLE_RATE_HZ, StereoFrame},
    sample::SampleLibrary,
    sampler::{Direction, EnvelopeParameters, LoopMode, Sampler, VoiceParameters, frame_to_i16},
    wav::Wave,
};

struct Options {
    input: PathBuf,
    output: PathBuf,
    pitch: f32,
    cents: f32,
    speed: f32,
    gain: f32,
    reverse: bool,
    loop_mode: LoopMode,
    voices: usize,
    seconds: f32,
    attack_ms: f32,
    decay_ms: f32,
    sustain: f32,
    release_ms: f32,
    sequence: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options()?;
    let input = fs::read(&options.input)?;
    let wave = Wave::parse(&input).map_err(|e| format!("unsupported WAV: {e:?}"))?;
    let mut memory = vec![0u8; wave.pcm().len()];
    let mut library = SampleLibrary::<1>::new(&mut memory);
    let sample_name = options
        .input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sample.wav");
    let sample_id = library
        .load(sample_name, &input)
        .map_err(|e| format!("load failed: {e:?}"))?;
    let mut sampler = Sampler::<128>::new();
    sampler.set_master_gain(0.8);
    let frames = (options.seconds.clamp(0.01, 600.0) * SAMPLE_RATE_HZ as f32) as usize;
    let mut rendered = Vec::with_capacity(frames * 2);
    let mut block = [StereoFrame::default(); 128];
    let mut next_sequence = 0usize;
    let sequence_interval = SAMPLE_RATE_HZ as usize / 4;
    let mut initially_triggered = false;
    let mut released = false;
    for first in (0..frames).step_by(block.len()) {
        if !initially_triggered {
            for voice in 0..options.voices.clamp(1, 128) {
                trigger(
                    &mut sampler,
                    &library,
                    sample_id,
                    &options,
                    (voice % 12) as f32 - 6.0,
                );
            }
            initially_triggered = true;
        }
        if options.sequence && !released && first >= next_sequence {
            let note = [0.0, 4.0, 7.0, 12.0][(first / sequence_interval) % 4];
            trigger(&mut sampler, &library, sample_id, &options, note);
            next_sequence = first + sequence_interval;
        }
        if !released && first >= frames * 3 / 4 {
            sampler.release_all();
            released = true;
        }
        let count = (frames - first).min(block.len());
        sampler.process(&library, &mut block[..count]);
        rendered.extend_from_slice(&block[..count]);
    }
    write_wav(&options.output, &rendered)?;
    let stats = sampler.statistics();
    let peak_left = rendered
        .iter()
        .fold(0.0f32, |peak, frame| peak.max(frame.left.abs()));
    let peak_right = rendered
        .iter()
        .fold(0.0f32, |peak, frame| peak.max(frame.right.abs()));
    println!(
        "rendered {} frames at {} Hz to {} (peak L {:.3} R {:.3}, clips {}, peak voices {})",
        frames,
        SAMPLE_RATE_HZ,
        options.output.display(),
        peak_left,
        peak_right,
        stats.clipping_samples,
        stats.peak_active_voices
    );
    Ok(())
}

fn trigger<const N: usize>(
    sampler: &mut Sampler<N>,
    library: &SampleLibrary<'_, 1>,
    id: u16,
    options: &Options,
    offset: f32,
) {
    let metadata = library.get(id).unwrap();
    let parameters = VoiceParameters {
        end_frame: metadata.frame_count,
        semitones: options.pitch + offset,
        cents: options.cents,
        speed: options.speed,
        gain: options.gain,
        direction: if options.reverse {
            Direction::Reverse
        } else {
            Direction::Forward
        },
        loop_mode: options.loop_mode,
        envelope: EnvelopeParameters {
            attack_ms: options.attack_ms,
            decay_ms: options.decay_ms,
            sustain: options.sustain,
            release_ms: options.release_ms,
            declick_ms: 2.0,
        },
        ..VoiceParameters::default()
    };
    let _ = sampler.trigger(library, id, parameters);
}

fn parse_options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or_else(usage)?;
    let output = args.next().ok_or_else(usage)?;
    let mut options = Options {
        input: input.into(),
        output: output.into(),
        pitch: 0.0,
        cents: 0.0,
        speed: 1.0,
        gain: 0.8,
        reverse: false,
        loop_mode: LoopMode::OneShot,
        voices: 1,
        seconds: 4.0,
        attack_ms: 2.0,
        decay_ms: 20.0,
        sustain: 1.0,
        release_ms: 50.0,
        sequence: false,
    };
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--pitch" => options.pitch = value(&mut args, &flag)?,
            "--cents" => options.cents = value(&mut args, &flag)?,
            "--speed" => options.speed = value(&mut args, &flag)?,
            "--gain" => options.gain = value(&mut args, &flag)?,
            "--voices" => options.voices = value(&mut args, &flag)?,
            "--seconds" => options.seconds = value(&mut args, &flag)?,
            "--attack-ms" => options.attack_ms = value(&mut args, &flag)?,
            "--decay-ms" => options.decay_ms = value(&mut args, &flag)?,
            "--sustain" => options.sustain = value(&mut args, &flag)?,
            "--release-ms" => options.release_ms = value(&mut args, &flag)?,
            "--reverse" => options.reverse = true,
            "--sequence" => options.sequence = true,
            "--loop" => {
                options.loop_mode = match args.next().as_deref() {
                    Some("forward") => LoopMode::Forward,
                    Some("pingpong") => LoopMode::PingPong,
                    Some("off") => LoopMode::OneShot,
                    _ => return Err("--loop expects off, forward, or pingpong".into()),
                }
            }
            _ => return Err(format!("unknown option {flag}\n{}", usage())),
        }
    }
    Ok(options)
}
fn value<T: std::str::FromStr>(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<T, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))?
        .parse()
        .map_err(|_| format!("invalid value for {flag}"))
}
fn usage() -> String {
    "usage: pie8-sampler-render INPUT.wav OUTPUT.wav [--pitch SEMITONES] [--cents N] [--speed RATIO] [--gain 0..4] [--reverse] [--loop off|forward|pingpong] [--voices 1..128] [--seconds N] [--attack-ms N] [--decay-ms N] [--sustain 0..1] [--release-ms N] [--sequence]".into()
}

fn write_wav(path: &PathBuf, frames: &[StereoFrame]) -> io::Result<()> {
    let data_bytes = u32::try_from(frames.len().saturating_mul(4))
        .map_err(|_| io::Error::other("output too large"))?;
    let mut file = fs::File::create(path)?;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&SAMPLE_RATE_HZ.to_le_bytes())?;
    file.write_all(&(SAMPLE_RATE_HZ * 4).to_le_bytes())?;
    file.write_all(&4u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;
    for frame in frames {
        file.write_all(&frame_to_i16(frame.left).to_le_bytes())?;
        file.write_all(&frame_to_i16(frame.right).to_le_bytes())?;
    }
    Ok(())
}

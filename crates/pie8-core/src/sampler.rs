//! Allocation-free sample voices, ADSR envelopes, deterministic voice allocation and mixing.

use crate::{
    audio::{SAMPLE_RATE_HZ, StereoFrame},
    sample::SampleId,
};

#[derive(Clone, Copy)]
pub struct SampleView<'a> {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub frame_count: u32,
    pub pcm: &'a [u8],
}

pub trait SampleSource {
    fn generation(&self) -> u32;
    fn sample(&self, id: SampleId) -> Option<SampleView<'_>>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopMode {
    OneShot,
    Forward,
    PingPong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvelopeParameters {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    pub declick_ms: f32,
}

impl Default for EnvelopeParameters {
    fn default() -> Self {
        Self {
            attack_ms: 2.0,
            decay_ms: 20.0,
            sustain: 1.0,
            release_ms: 10.0,
            declick_ms: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    stage: EnvelopeStage,
    level: f32,
    step: f32,
    remaining: u32,
    parameters: EnvelopeParameters,
}

impl Envelope {
    pub fn new(parameters: EnvelopeParameters) -> Self {
        let mut envelope = Self {
            stage: EnvelopeStage::Idle,
            level: 0.0,
            step: 0.0,
            remaining: 0,
            parameters: sanitize_envelope(parameters),
        };
        envelope.reset();
        envelope
    }
    pub fn trigger(&mut self) {
        self.stage = EnvelopeStage::Attack;
        let frames = duration_frames(self.parameters.attack_ms.max(self.parameters.declick_ms));
        self.begin_segment(1.0, frames);
    }
    pub fn release(&mut self) {
        if self.stage == EnvelopeStage::Idle {
            return;
        }
        self.stage = EnvelopeStage::Release;
        let frames = duration_frames(self.parameters.release_ms.max(self.parameters.declick_ms));
        self.begin_segment(0.0, frames);
    }
    pub fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.level = 0.0;
        self.step = 0.0;
        self.remaining = 0;
    }
    pub const fn stage(&self) -> EnvelopeStage {
        self.stage
    }
    pub const fn level(&self) -> f32 {
        self.level
    }
    pub fn next_level(&mut self) -> f32 {
        let output = finite(self.level).clamp(0.0, 1.0);
        if self.remaining != 0 {
            self.level += self.step;
            self.remaining -= 1;
        }
        if self.remaining == 0 {
            match self.stage {
                EnvelopeStage::Attack => {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.begin_segment(
                        self.parameters.sustain,
                        duration_frames(self.parameters.decay_ms),
                    );
                }
                EnvelopeStage::Decay => {
                    self.level = self.parameters.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
                EnvelopeStage::Release => self.reset(),
                _ => {}
            }
        }
        output
    }
    fn begin_segment(&mut self, target: f32, frames: u32) {
        self.remaining = frames.max(1);
        self.step = (target - self.level) / self.remaining as f32;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceParameters {
    pub start_frame: u32,
    pub end_frame: u32,
    pub speed: f32,
    pub semitones: f32,
    pub cents: f32,
    pub direction: Direction,
    pub loop_mode: LoopMode,
    pub gain: f32,
    pub pan: f32,
    pub envelope: EnvelopeParameters,
}

impl Default for VoiceParameters {
    fn default() -> Self {
        Self {
            start_frame: 0,
            end_frame: u32::MAX,
            speed: 1.0,
            semitones: 0.0,
            cents: 0.0,
            direction: Direction::Forward,
            loop_mode: LoopMode::OneShot,
            gain: 1.0,
            pan: 0.0,
            envelope: EnvelopeParameters::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Voice {
    active: bool,
    sample_id: SampleId,
    generation: u32,
    position: f64,
    increment: f64,
    start: u32,
    end: u32,
    direction: Direction,
    loop_mode: LoopMode,
    gain: f32,
    pan: f32,
    envelope: Envelope,
    age: u64,
}

impl Voice {
    pub fn inactive() -> Self {
        Self {
            active: false,
            sample_id: 0,
            generation: 0,
            position: 0.0,
            increment: 0.0,
            start: 0,
            end: 0,
            direction: Direction::Forward,
            loop_mode: LoopMode::OneShot,
            gain: 0.0,
            pan: 0.0,
            envelope: Envelope::new(EnvelopeParameters::default()),
            age: 0,
        }
    }
    pub const fn is_active(&self) -> bool {
        self.active
    }
    pub const fn position(&self) -> f64 {
        self.position
    }
    pub const fn direction(&self) -> Direction {
        self.direction
    }
    pub const fn envelope_stage(&self) -> EnvelopeStage {
        self.envelope.stage()
    }
    fn trigger(
        &mut self,
        source: &impl SampleSource,
        id: SampleId,
        parameters: VoiceParameters,
        age: u64,
    ) -> bool {
        let Some(sample) = source.sample(id) else {
            self.active = false;
            return false;
        };
        let start = parameters.start_frame.min(sample.frame_count);
        let end = parameters.end_frame.min(sample.frame_count);
        if end <= start {
            self.active = false;
            return false;
        }
        let pitch = (parameters.semitones.clamp(-96.0, 96.0)
            + parameters.cents.clamp(-100.0, 100.0) / 100.0)
            / 12.0;
        let speed = finite(parameters.speed).abs().clamp(0.001, 64.0);
        self.increment =
            f64::from(speed * fast_exp2(pitch) * sample.sample_rate as f32 / SAMPLE_RATE_HZ as f32);
        self.active = true;
        self.sample_id = id;
        self.generation = source.generation();
        self.start = start;
        self.end = end;
        self.direction = parameters.direction;
        self.loop_mode = parameters.loop_mode;
        self.gain = finite(parameters.gain).clamp(0.0, 4.0);
        self.pan = finite(parameters.pan).clamp(-1.0, 1.0);
        self.age = age;
        self.position = match self.direction {
            Direction::Forward => f64::from(start),
            Direction::Reverse => f64::from(end - 1),
        };
        self.envelope = Envelope::new(parameters.envelope);
        self.envelope.trigger();
        true
    }
    fn render(&mut self, source: &impl SampleSource) -> StereoFrame {
        if !self.active || self.generation != source.generation() {
            self.active = false;
            return StereoFrame::default();
        }
        let Some(sample) = source.sample(self.sample_id) else {
            self.active = false;
            return StereoFrame::default();
        };
        if self.position < f64::from(self.start) || self.position >= f64::from(self.end) {
            self.active = false;
            return StereoFrame::default();
        }
        let base = self.position as u32;
        let fraction = (self.position - f64::from(base)) as f32;
        let next = (base + 1).min(self.end - 1);
        let a = decode_frame(sample, base);
        let b = decode_frame(sample, next);
        let amplitude = self.envelope.next_level() * self.gain;
        let frame = StereoFrame {
            left: (a.left + (b.left - a.left) * fraction) * amplitude * pan_left(self.pan),
            right: (a.right + (b.right - a.right) * fraction) * amplitude * pan_right(self.pan),
        };
        if self.envelope.stage() == EnvelopeStage::Idle {
            self.active = false;
        }
        self.advance();
        frame
    }
    fn advance(&mut self) {
        match self.direction {
            Direction::Forward => self.position += self.increment,
            Direction::Reverse => self.position -= self.increment,
        }
        let start = f64::from(self.start);
        let end = f64::from(self.end);
        let span = end - start;
        if span <= 0.0 {
            self.active = false;
            return;
        }
        match self.loop_mode {
            LoopMode::OneShot => {
                if self.position < start || self.position >= end {
                    self.active = false;
                }
            }
            LoopMode::Forward => {
                self.position = start + (self.position - start) % span;
                if self.position < start {
                    self.position += span;
                }
            }
            LoopMode::PingPong => {
                let period = span * 2.0;
                let mut phase = (self.position - start) % period;
                if phase < 0.0 {
                    phase += period;
                }
                if phase < span {
                    self.position = start + phase;
                    self.direction = Direction::Forward;
                } else {
                    self.position = end - 1.0 - (phase - span);
                    self.direction = Direction::Reverse;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MixerStatistics {
    pub peak_left: f32,
    pub peak_right: f32,
    pub clipping_samples: u64,
    pub active_voices: usize,
    pub peak_active_voices: usize,
}

pub struct Sampler<const N: usize> {
    voices: [Voice; N],
    age: u64,
    master_gain: f32,
    statistics: MixerStatistics,
}

impl<const N: usize> Sampler<N> {
    pub fn new() -> Self {
        Self {
            voices: [Voice::inactive(); N],
            age: 0,
            master_gain: 0.8,
            statistics: MixerStatistics::default(),
        }
    }
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = finite(gain).clamp(0.0, 4.0);
    }
    pub fn trigger(
        &mut self,
        source: &impl SampleSource,
        id: SampleId,
        parameters: VoiceParameters,
    ) -> Option<usize> {
        self.age = self.age.wrapping_add(1);
        let index = self.voices.iter().position(|v| !v.active).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.age)
                .map(|(i, _)| i)
        })?;
        self.voices[index]
            .trigger(source, id, parameters, self.age)
            .then_some(index)
    }
    pub fn release(&mut self, voice: usize) {
        if let Some(voice) = self.voices.get_mut(voice) {
            voice.envelope.release();
        }
    }
    pub fn release_all(&mut self) {
        for voice in &mut self.voices {
            voice.envelope.release();
        }
    }
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
        }
    }
    pub fn voice(&self, index: usize) -> Option<&Voice> {
        self.voices.get(index)
    }
    pub const fn statistics(&self) -> MixerStatistics {
        self.statistics
    }
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }
    pub fn process(&mut self, source: &impl SampleSource, output: &mut [StereoFrame]) {
        self.statistics.peak_left = 0.0;
        self.statistics.peak_right = 0.0;
        for out in output {
            let mut left = 0.0;
            let mut right = 0.0;
            for voice in &mut self.voices {
                let frame = voice.render(source);
                left += frame.left;
                right += frame.right;
            }
            left = finite(left * self.master_gain);
            right = finite(right * self.master_gain);
            self.statistics.peak_left = self.statistics.peak_left.max(left.abs());
            self.statistics.peak_right = self.statistics.peak_right.max(right.abs());
            if left.abs() > 1.0 {
                self.statistics.clipping_samples += 1;
            }
            if right.abs() > 1.0 {
                self.statistics.clipping_samples += 1;
            }
            *out = StereoFrame {
                left: left.clamp(-1.0, 1.0),
                right: right.clamp(-1.0, 1.0),
            };
        }
        self.statistics.active_voices = self.voices.iter().filter(|v| v.active).count();
        self.statistics.peak_active_voices = self
            .statistics
            .peak_active_voices
            .max(self.statistics.active_voices);
    }
}
impl<const N: usize> Default for Sampler<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn frame_to_i16(value: f32) -> i16 {
    let scaled = finite(value).clamp(-1.0, 1.0) * 32767.0;
    (scaled + if scaled >= 0.0 { 0.5 } else { -0.5 }) as i16
}

fn decode_frame(sample: SampleView<'_>, frame: u32) -> StereoFrame {
    if frame >= sample.frame_count {
        return StereoFrame::default();
    }
    let bytes = usize::from(sample.bits_per_sample / 8);
    let offset = frame as usize * usize::from(sample.channels) * bytes;
    let left = decode_pcm(&sample.pcm[offset..], sample.bits_per_sample);
    let right = if sample.channels == 2 {
        decode_pcm(&sample.pcm[offset + bytes..], sample.bits_per_sample)
    } else {
        left
    };
    StereoFrame { left, right }
}
fn decode_pcm(bytes: &[u8], bits: u16) -> f32 {
    if bits == 16 && bytes.len() >= 2 {
        i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32768.0
    } else if bits == 24 && bytes.len() >= 3 {
        let value =
            (i32::from(bytes[0]) | (i32::from(bytes[1]) << 8) | (i32::from(bytes[2]) << 16)) << 8;
        value as f32 / 2_147_483_648.0
    } else {
        0.0
    }
}
fn pan_left(pan: f32) -> f32 {
    if pan > 0.0 { 1.0 - pan } else { 1.0 }
}
fn pan_right(pan: f32) -> f32 {
    if pan < 0.0 { 1.0 + pan } else { 1.0 }
}
fn duration_frames(ms: f32) -> u32 {
    ((finite(ms).clamp(0.0, 60_000.0) * SAMPLE_RATE_HZ as f32 / 1000.0 + 0.5) as u32).max(1)
}
fn sanitize_envelope(p: EnvelopeParameters) -> EnvelopeParameters {
    EnvelopeParameters {
        attack_ms: finite(p.attack_ms).clamp(0.0, 60_000.0),
        decay_ms: finite(p.decay_ms).clamp(0.0, 60_000.0),
        sustain: finite(p.sustain).clamp(0.0, 1.0),
        release_ms: finite(p.release_ms).clamp(0.0, 60_000.0),
        declick_ms: finite(p.declick_ms).clamp(0.0, 20.0),
    }
}
fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
fn fast_exp2(value: f32) -> f32 {
    let x = finite(value).clamp(-16.0, 16.0);
    let truncated = x as i32;
    let whole = if x < truncated as f32 {
        truncated - 1
    } else {
        truncated
    };
    let f = x - whole as f32;
    let p = 1.0 + f * (core::f32::consts::LN_2 + f * (0.240_226_5 + f * 0.055_504_1));
    f32::from_bits(((whole + 127) as u32) << 23) * p
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        pcm: [u8; 16],
        rate: u32,
        generation: u32,
    }
    impl SampleSource for Source {
        fn generation(&self) -> u32 {
            self.generation
        }
        fn sample(&self, id: SampleId) -> Option<SampleView<'_>> {
            (id == 0).then_some(SampleView {
                channels: 1,
                sample_rate: self.rate,
                bits_per_sample: 16,
                frame_count: 8,
                pcm: &self.pcm,
            })
        }
    }
    fn source() -> Source {
        let mut pcm = [0; 16];
        for (i, c) in pcm.chunks_exact_mut(2).enumerate() {
            c.copy_from_slice(&((i as i16) * 4000).to_le_bytes())
        }
        Source {
            pcm,
            rate: 48_000,
            generation: 1,
        }
    }
    #[test]
    fn interpolation_and_duration_are_block_independent() {
        let s = source();
        let mut a = Sampler::<2>::new();
        let p = VoiceParameters {
            speed: 0.5,
            envelope: EnvelopeParameters {
                attack_ms: 0.0,
                declick_ms: 0.0,
                ..EnvelopeParameters::default()
            },
            ..VoiceParameters::default()
        };
        a.trigger(&s, 0, p);
        let mut all = [StereoFrame::default(); 16];
        a.process(&s, &mut all);
        let mut b = Sampler::<2>::new();
        b.trigger(&s, 0, p);
        let mut x = [StereoFrame::default(); 3];
        let mut y = [StereoFrame::default(); 13];
        b.process(&s, &mut x);
        b.process(&s, &mut y);
        assert_eq!(&all[..3], &x);
        assert_eq!(&all[3..], &y);
        assert!(all[3].left > all[2].left)
    }
    #[test]
    fn source_rate_changes_duration() {
        let mut s = source();
        s.rate = 24_000;
        let mut e = Sampler::<1>::new();
        let mut p = VoiceParameters::default();
        p.envelope.attack_ms = 0.0;
        p.envelope.declick_ms = 0.0;
        e.trigger(&s, 0, p);
        let mut out = [StereoFrame::default(); 16];
        e.process(&s, &mut out);
        assert_eq!(e.statistics().active_voices, 0)
    }
    #[test]
    fn reverse_and_loops_stay_bounded() {
        let s = source();
        for mode in [LoopMode::Forward, LoopMode::PingPong] {
            let mut e = Sampler::<1>::new();
            let p = VoiceParameters {
                direction: Direction::Reverse,
                loop_mode: mode,
                start_frame: 2,
                end_frame: 5,
                envelope: EnvelopeParameters {
                    attack_ms: 0.0,
                    declick_ms: 0.0,
                    ..EnvelopeParameters::default()
                },
                ..VoiceParameters::default()
            };
            e.trigger(&s, 0, p);
            let mut out = [StereoFrame::default(); 100];
            e.process(&s, &mut out);
            assert_eq!(e.statistics().active_voices, 1)
        }
    }
    #[test]
    fn envelope_transitions_and_release() {
        let mut e = Envelope::new(EnvelopeParameters {
            attack_ms: 1.0,
            decay_ms: 1.0,
            sustain: 0.5,
            release_ms: 1.0,
            declick_ms: 0.0,
        });
        e.trigger();
        for _ in 0..96 {
            e.next_level();
        }
        assert_eq!(e.stage(), EnvelopeStage::Sustain);
        e.release();
        for _ in 0..48 {
            e.next_level();
        }
        assert_eq!(e.stage(), EnvelopeStage::Idle)
    }
    #[test]
    fn allocation_steals_oldest_and_generation_invalidates() {
        let mut s = source();
        let mut e = Sampler::<2>::new();
        let p = VoiceParameters::default();
        assert_eq!(e.trigger(&s, 0, p), Some(0));
        assert_eq!(e.trigger(&s, 0, p), Some(1));
        assert_eq!(e.trigger(&s, 0, p), Some(0));
        s.generation = 2;
        let mut out = [StereoFrame::default(); 1];
        e.process(&s, &mut out);
        assert_eq!(e.statistics().active_voices, 0)
    }
    #[test]
    fn mixing_clips_only_at_output_and_rejects_nan() {
        let s = source();
        let mut e = Sampler::<8>::new();
        let p = VoiceParameters {
            gain: 4.0,
            envelope: EnvelopeParameters {
                attack_ms: 0.0,
                declick_ms: 0.0,
                ..EnvelopeParameters::default()
            },
            ..VoiceParameters::default()
        };
        for _ in 0..8 {
            e.trigger(&s, 0, p);
        }
        let mut out = [StereoFrame::default(); 8];
        e.process(&s, &mut out);
        assert!(
            out.iter()
                .all(|f| f.left.is_finite() && f.left.abs() <= 1.0)
        );
        assert!(e.statistics().clipping_samples > 0);
        assert_eq!(frame_to_i16(f32::NAN), 0)
    }
}

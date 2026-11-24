pub const SAMPLE_RATE_HZ: u32 = 48_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Waveform {
    Silence,
    Sine,
    Triangle,
    Square,
}

pub trait AudioProcessor {
    /// Fills an arbitrary-size block without allocation.
    fn process(&mut self, output: &mut [StereoFrame]);
}

pub struct SignalGenerator {
    waveform: Waveform,
    frequency_hz: f32,
    gain: f32,
    phase: f32,
}

impl SignalGenerator {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            waveform: Waveform::Silence,
            frequency_hz: 440.0,
            gain: 0.0,
            phase: 0.0,
        }
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }
    pub fn set_frequency_hz(&mut self, frequency_hz: f32) {
        self.frequency_hz = frequency_hz.clamp(1.0, SAMPLE_RATE_HZ as f32 * 0.45);
    }
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }
    #[must_use]
    pub const fn waveform(&self) -> Waveform {
        self.waveform
    }
    #[must_use]
    pub const fn frequency_hz(&self) -> f32 {
        self.frequency_hz
    }
    #[must_use]
    pub const fn gain(&self) -> f32 {
        self.gain
    }

    fn sample(&self) -> f32 {
        match self.waveform {
            Waveform::Silence => 0.0,
            Waveform::Sine => fast_sine(self.phase),
            Waveform::Triangle => 1.0 - 4.0 * (self.phase - 0.5).abs(),
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

impl Default for SignalGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for SignalGenerator {
    fn process(&mut self, output: &mut [StereoFrame]) {
        let phase_step = self.frequency_hz / SAMPLE_RATE_HZ as f32;
        for frame in output {
            let value = (self.sample() * self.gain).clamp(-1.0, 1.0);
            *frame = StereoFrame {
                left: value,
                right: value,
            };
            self.phase += phase_step;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }
}

// Allocation-free parabolic approximation with a correction term. The maximum
// error is suitable for a diagnostic tone, not a final synthesizer oscillator.
fn fast_sine(phase: f32) -> f32 {
    let mut x = phase * 2.0;
    if x > 1.0 {
        x -= 2.0;
    }
    let y = 4.0 * x * (1.0 - x.abs());
    0.225 * (y * y.abs() - y) + y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_default_for_any_block_size() {
        let mut generator = SignalGenerator::new();
        let mut block = [StereoFrame {
            left: 1.0,
            right: -1.0,
        }; 17];
        generator.process(&mut block);
        assert!(block.iter().all(|frame| *frame == StereoFrame::default()));
    }

    #[test]
    fn square_wave_is_stereo_and_clipped_to_configured_gain() {
        let mut generator = SignalGenerator::new();
        generator.set_waveform(Waveform::Square);
        generator.set_gain(2.0);
        let mut block = [StereoFrame::default(); 64];
        generator.process(&mut block);
        assert!(block.iter().all(|frame| frame.left == frame.right));
        assert!(block.iter().all(|frame| frame.left.abs() <= 1.0));
        assert_eq!(generator.gain(), 1.0);
    }

    #[test]
    fn phase_is_continuous_between_configurable_blocks() {
        let mut one = SignalGenerator::new();
        let mut two = SignalGenerator::new();
        one.set_waveform(Waveform::Triangle);
        two.set_waveform(Waveform::Triangle);
        one.set_gain(0.5);
        two.set_gain(0.5);
        let mut complete = [StereoFrame::default(); 32];
        let mut first = [StereoFrame::default(); 7];
        let mut second = [StereoFrame::default(); 25];
        one.process(&mut complete);
        two.process(&mut first);
        two.process(&mut second);
        assert_eq!(&complete[..7], &first);
        assert_eq!(&complete[7..], &second);
    }

    #[test]
    fn sine_has_expected_quadrant_signs() {
        assert!(fast_sine(0.25) > 0.99);
        assert!(fast_sine(0.75) < -0.99);
    }
}

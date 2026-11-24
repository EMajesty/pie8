//! Allocation-free RIFF/WAVE PCM parser and float conversion.
use crate::audio::StereoFrame;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaveError {
    Truncated,
    InvalidRiff,
    InvalidChunk,
    UnsupportedEncoding,
    UnsupportedChannels,
    UnsupportedBits,
    UnsupportedRate,
    MissingFormat,
    MissingData,
    Overflow,
}

#[derive(Clone, Copy, Debug)]
pub struct Wave<'a> {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub frame_count: u32,
    data: &'a [u8],
}
impl<'a> Wave<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, WaveError> {
        if bytes.len() < 12 {
            return Err(WaveError::Truncated);
        }
        if &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(WaveError::InvalidRiff);
        }
        let declared = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let riff_end = declared.checked_add(8).ok_or(WaveError::Overflow)?;
        if riff_end > bytes.len() {
            return Err(WaveError::Truncated);
        }
        let mut offset = 12;
        let mut format = None;
        let mut data = None;
        while offset < riff_end {
            if offset.checked_add(8).ok_or(WaveError::Overflow)? > riff_end {
                return Err(WaveError::Truncated);
            }
            let id = &bytes[offset..offset + 4];
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let start = offset + 8;
            let end = start.checked_add(length).ok_or(WaveError::Overflow)?;
            if end > riff_end {
                return Err(WaveError::Truncated);
            }
            if id == b"fmt " {
                if length < 16 {
                    return Err(WaveError::InvalidChunk);
                }
                format = Some((
                    u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                    u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
                ))
            } else if id == b"data" {
                data = Some(&bytes[start..end])
            }
            offset = end.checked_add(length & 1).ok_or(WaveError::Overflow)?;
            if offset > riff_end {
                return Err(WaveError::Truncated);
            }
        }
        let (encoding, channels, rate, bits) = format.ok_or(WaveError::MissingFormat)?;
        if encoding != 1 {
            return Err(WaveError::UnsupportedEncoding);
        }
        if !matches!(channels, 1 | 2) {
            return Err(WaveError::UnsupportedChannels);
        }
        if !matches!(bits, 16 | 24) {
            return Err(WaveError::UnsupportedBits);
        }
        if rate != 48_000 {
            return Err(WaveError::UnsupportedRate);
        }
        let data = data.ok_or(WaveError::MissingData)?;
        let frame_bytes = usize::from(channels) * usize::from(bits / 8);
        if data.len() % frame_bytes != 0 {
            return Err(WaveError::InvalidChunk);
        }
        let frames = u32::try_from(data.len() / frame_bytes).map_err(|_| WaveError::Overflow)?;
        Ok(Self {
            channels,
            sample_rate: rate,
            bits_per_sample: bits,
            frame_count: frames,
            data,
        })
    }
    pub fn pcm(&self) -> &'a [u8] {
        self.data
    }
    pub fn duration_ms(&self) -> u64 {
        u64::from(self.frame_count) * 1000 / u64::from(self.sample_rate)
    }
    pub fn frame(&self, index: u32) -> Option<StereoFrame> {
        if index >= self.frame_count {
            return None;
        }
        let bytes = usize::from(self.bits_per_sample / 8);
        let offset = index as usize * usize::from(self.channels) * bytes;
        let left = decode(&self.data[offset..], self.bits_per_sample);
        let right = if self.channels == 2 {
            decode(&self.data[offset + bytes..], self.bits_per_sample)
        } else {
            left
        };
        Some(StereoFrame { left, right })
    }
}
fn decode(bytes: &[u8], bits: u16) -> f32 {
    if bits == 16 {
        i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32768.0
    } else {
        let raw =
            (i32::from(bytes[0]) | (i32::from(bytes[1]) << 8) | (i32::from(bytes[2]) << 16)) << 8;
        raw as f32 / 2_147_483_648.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wave(bits: u16, channels: u16, data: &[u8], junk: bool) -> std::vec::Vec<u8> {
        let mut chunks = std::vec::Vec::new();
        if junk {
            chunks.extend_from_slice(b"JUNK\x01\0\0\0x\0")
        }
        chunks.extend_from_slice(b"data");
        chunks.extend_from_slice(&(data.len() as u32).to_le_bytes());
        chunks.extend_from_slice(data);
        if data.len() % 2 == 1 {
            chunks.push(0)
        }
        chunks.extend_from_slice(b"fmt \x10\0\0\0\x01\0");
        chunks.extend_from_slice(&channels.to_le_bytes());
        chunks.extend_from_slice(&48_000u32.to_le_bytes());
        chunks.extend_from_slice(&0u32.to_le_bytes());
        chunks.extend_from_slice(&0u16.to_le_bytes());
        chunks.extend_from_slice(&bits.to_le_bytes());
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&((chunks.len() + 4) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(&chunks);
        out
    }
    #[test]
    fn parses_noncanonical_chunks_and_pcm16() {
        let b = wave(16, 1, &[0, 0, 0xff, 0x7f], true);
        let w = Wave::parse(&b).unwrap();
        assert_eq!(w.frame_count, 2);
        assert!(w.frame(1).unwrap().left > 0.99);
        assert_eq!(w.frame(1).unwrap().left, w.frame(1).unwrap().right)
    }
    #[test]
    fn converts_signed_pcm24() {
        let b = wave(24, 2, &[0, 0, 0x80, 0xff, 0xff, 0x7f], false);
        let w = Wave::parse(&b).unwrap();
        let f = w.frame(0).unwrap();
        assert_eq!(f.left, -1.0);
        assert!(f.right > 0.99)
    }
    #[test]
    fn rejects_rate_truncation_and_sizes() {
        let mut b = wave(16, 1, &[0, 0], false);
        let rate = b.windows(4).position(|x| x == b"fmt ").unwrap() + 12;
        b[rate..rate + 4].copy_from_slice(&44_100u32.to_le_bytes());
        assert_eq!(Wave::parse(&b).unwrap_err(), WaveError::UnsupportedRate);
        b.truncate(14);
        assert_eq!(Wave::parse(&b).unwrap_err(), WaveError::Truncated)
    }
}

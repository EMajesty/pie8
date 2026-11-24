use crate::{
    arena::{Arena, ArenaError},
    wav::{Wave, WaveError},
};
pub type SampleId = u16;
#[derive(Clone, Copy, Debug)]
pub struct SampleMetadata {
    pub id: SampleId,
    name: [u8; 64],
    name_len: u8,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub memory_bytes: u32,
    offset: u32,
}
impl SampleMetadata {
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleError {
    LibraryFull,
    NameTooLong,
    Wave(WaveError),
    Arena(ArenaError),
    TooLarge,
}
pub struct SampleLibrary<'a, const N: usize> {
    arena: Arena<'a>,
    samples: [Option<SampleMetadata>; N],
    count: usize,
    generation: u32,
}
impl<'a, const N: usize> SampleLibrary<'a, N> {
    pub fn new(memory: &'a mut [u8]) -> Self {
        Self {
            arena: Arena::new(memory),
            samples: [None; N],
            count: 0,
            generation: 1,
        }
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn used_bytes(&self) -> usize {
        self.arena.used()
    }
    pub fn capacity_bytes(&self) -> usize {
        self.arena.capacity()
    }
    pub const fn generation(&self) -> u32 {
        self.generation
    }
    pub fn get(&self, id: SampleId) -> Option<&SampleMetadata> {
        self.samples.get(id as usize)?.as_ref()
    }
    pub fn pcm(&self, id: SampleId) -> Option<&[u8]> {
        let s = self.get(id)?;
        self.arena
            .as_slice()
            .get(s.offset as usize..s.offset as usize + s.memory_bytes as usize)
    }
    pub fn load(&mut self, name: &str, wave_bytes: &[u8]) -> Result<SampleId, SampleError> {
        if self.count >= N {
            return Err(SampleError::LibraryFull);
        }
        if name.len() > 64 {
            return Err(SampleError::NameTooLong);
        }
        let wave = Wave::parse(wave_bytes).map_err(SampleError::Wave)?;
        let size = wave.pcm().len();
        let base = self.arena.allocate_base() as usize;
        let storage = self.arena.allocate(size, 4).map_err(SampleError::Arena)?;
        let offset = storage.as_ptr() as usize - base;
        storage.copy_from_slice(wave.pcm());
        let mut filename = [0; 64];
        filename[..name.len()].copy_from_slice(name.as_bytes());
        let id = self.count as SampleId;
        self.samples[self.count] = Some(SampleMetadata {
            id,
            name: filename,
            name_len: name.len() as u8,
            channels: wave.channels,
            sample_rate: wave.sample_rate,
            bits_per_sample: wave.bits_per_sample,
            frame_count: wave.frame_count,
            duration_ms: wave.duration_ms(),
            memory_bytes: u32::try_from(size).map_err(|_| SampleError::TooLarge)?,
            offset: u32::try_from(offset).map_err(|_| SampleError::TooLarge)?,
        });
        self.count += 1;
        Ok(id)
    }
    pub fn reset(&mut self) {
        self.samples.fill(None);
        self.count = 0;
        self.arena.reset();
        self.generation = self.generation.wrapping_add(1).max(1);
    }
}

impl<const N: usize> crate::sampler::SampleSource for SampleLibrary<'_, N> {
    fn generation(&self) -> u32 {
        self.generation
    }

    fn sample(&self, id: SampleId) -> Option<crate::sampler::SampleView<'_>> {
        let metadata = *self.get(id)?;
        Some(crate::sampler::SampleView {
            channels: metadata.channels,
            sample_rate: metadata.sample_rate,
            bits_per_sample: metadata.bits_per_sample,
            frame_count: metadata.frame_count,
            pcm: self.pcm(id)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wav() -> [u8; 46] {
        let mut b = [0; 46];
        b[..4].copy_from_slice(b"RIFF");
        b[4..8].copy_from_slice(&38u32.to_le_bytes());
        b[8..12].copy_from_slice(b"WAVE");
        b[12..16].copy_from_slice(b"fmt ");
        b[16..20].copy_from_slice(&16u32.to_le_bytes());
        b[20..22].copy_from_slice(&1u16.to_le_bytes());
        b[22..24].copy_from_slice(&1u16.to_le_bytes());
        b[24..28].copy_from_slice(&48_000u32.to_le_bytes());
        b[34..36].copy_from_slice(&16u16.to_le_bytes());
        b[36..40].copy_from_slice(b"data");
        b[40..44].copy_from_slice(&2u32.to_le_bytes());
        b
    }
    #[test]
    fn loads_metadata_and_resets() {
        let mut m = [0; 16];
        let mut l = SampleLibrary::<2>::new(&mut m);
        let id = l.load("kick.wav", &wav()).unwrap();
        assert_eq!(l.get(id).unwrap().frame_count, 1);
        assert_eq!(l.pcm(id).unwrap().len(), 2);
        let generation = l.generation();
        l.reset();
        assert_eq!(l.count(), 0);
        assert_eq!(l.used_bytes(), 0);
        assert_ne!(l.generation(), generation)
    }
    #[test]
    fn reports_exhaustion() {
        let mut m = [0; 1];
        let mut l = SampleLibrary::<1>::new(&mut m);
        assert!(matches!(
            l.load("x.wav", &wav()),
            Err(SampleError::Arena(ArenaError::Exhausted))
        ))
    }
}

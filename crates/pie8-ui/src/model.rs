pub const TRACKS: usize = 8;
pub const SONG_ROWS: usize = 64;
pub const STEPS: usize = 16;
pub const MAX_CHAINS: usize = 64;
pub const MAX_PHRASES: usize = 64;
pub const MAX_INSTRUMENTS: usize = 16;

pub type ObjectId = u8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Note {
    #[default]
    Empty,
    Off,
    Pitch(u8),
}

impl Note {
    pub const MAX: u8 = 119;

    #[must_use]
    pub fn transpose(self, semitones: i8) -> Self {
        match self {
            Self::Pitch(value) => {
                Self::Pitch(value.saturating_add_signed(semitones).min(Self::MAX))
            }
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Effect {
    pub command: Option<u8>,
    pub value: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhraseStep {
    pub note: Note,
    pub instrument: Option<ObjectId>,
    pub effects: [Effect; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phrase {
    pub steps: [PhraseStep; STEPS],
}

impl Default for Phrase {
    fn default() -> Self {
        Self {
            steps: [PhraseStep::default(); STEPS],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChainStep {
    pub phrase: Option<ObjectId>,
    pub transpose: i8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chain {
    pub steps: [ChainStep; STEPS],
}

impl Default for Chain {
    fn default() -> Self {
        Self {
            steps: [ChainStep::default(); STEPS],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopMode {
    OneShot,
    Forward,
    PingPong,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplerInstrument {
    pub sample: Option<u16>,
    pub gain: f32,
    pub pan: f32,
    pub semitones: i8,
    pub cents: i16,
    pub direction: Direction,
    pub loop_mode: LoopMode,
    pub start_frame: u32,
    pub end_frame: u32,
    pub attack_ms: u16,
    pub decay_ms: u16,
    pub sustain: u8,
    pub release_ms: u16,
}

impl Default for SamplerInstrument {
    fn default() -> Self {
        Self {
            sample: None,
            gain: 0.8,
            pan: 0.0,
            semitones: 0,
            cents: 0,
            direction: Direction::Forward,
            loop_mode: LoopMode::OneShot,
            start_frame: 0,
            end_frame: 0,
            attack_ms: 2,
            decay_ms: 20,
            sustain: 255,
            release_ms: 50,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Instrument {
    pub sampler: SamplerInstrument,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelError {
    Full,
    InvalidChain,
    InvalidPhrase,
    InvalidInstrument,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub song: [[Option<ObjectId>; TRACKS]; SONG_ROWS],
    pub chains: [Option<Chain>; MAX_CHAINS],
    pub phrases: [Option<Phrase>; MAX_PHRASES],
    pub instruments: [Option<Instrument>; MAX_INSTRUMENTS],
}

impl Default for Project {
    fn default() -> Self {
        Self {
            song: [[None; TRACKS]; SONG_ROWS],
            chains: [None; MAX_CHAINS],
            phrases: [None; MAX_PHRASES],
            instruments: [None; MAX_INSTRUMENTS],
        }
    }
}

impl Project {
    pub fn allocate_chain(&mut self) -> Result<ObjectId, ModelError> {
        allocate(&mut self.chains, Chain::default())
    }

    pub fn allocate_phrase(&mut self) -> Result<ObjectId, ModelError> {
        allocate(&mut self.phrases, Phrase::default())
    }

    pub fn allocate_instrument(&mut self) -> Result<ObjectId, ModelError> {
        allocate(&mut self.instruments, Instrument::default())
    }

    pub fn clone_chain(&mut self, source: ObjectId) -> Result<ObjectId, ModelError> {
        let value = self
            .chains
            .get(source as usize)
            .and_then(|value| *value)
            .ok_or(ModelError::InvalidChain)?;
        allocate(&mut self.chains, value)
    }

    pub fn clone_phrase(&mut self, source: ObjectId) -> Result<ObjectId, ModelError> {
        let value = self
            .phrases
            .get(source as usize)
            .and_then(|value| *value)
            .ok_or(ModelError::InvalidPhrase)?;
        allocate(&mut self.phrases, value)
    }

    #[must_use]
    pub fn chain_references(&self, id: ObjectId) -> usize {
        self.song
            .iter()
            .flatten()
            .filter(|cell| **cell == Some(id))
            .count()
    }

    #[must_use]
    pub fn phrase_references(&self, id: ObjectId) -> usize {
        self.chains
            .iter()
            .flatten()
            .flat_map(|chain| chain.steps)
            .filter(|step| step.phrase == Some(id))
            .count()
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        for id in self.song.iter().flatten().flatten() {
            if self
                .chains
                .get(*id as usize)
                .and_then(|value| *value)
                .is_none()
            {
                return Err(ModelError::InvalidChain);
            }
        }
        for chain in self.chains.iter().flatten() {
            for id in chain.steps.iter().filter_map(|step| step.phrase) {
                if self
                    .phrases
                    .get(id as usize)
                    .and_then(|value| *value)
                    .is_none()
                {
                    return Err(ModelError::InvalidPhrase);
                }
            }
        }
        for phrase in self.phrases.iter().flatten() {
            for id in phrase.steps.iter().filter_map(|step| step.instrument) {
                if self
                    .instruments
                    .get(id as usize)
                    .and_then(|value| *value)
                    .is_none()
                {
                    return Err(ModelError::InvalidInstrument);
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn example() -> Self {
        let mut project = Self::default();
        project.instruments[0] = Some(Instrument::default());
        project.instruments[1] = Some(Instrument {
            sampler: SamplerInstrument {
                sample: Some(1),
                gain: 0.65,
                pan: -0.3,
                semitones: -12,
                loop_mode: LoopMode::Forward,
                end_frame: 48_000,
                attack_ms: 5,
                release_ms: 180,
                ..SamplerInstrument::default()
            },
        });
        for id in 0..6 {
            let mut phrase = Phrase::default();
            for row in (0..STEPS).step_by(4) {
                phrase.steps[row].note = Note::Pitch(36 + id as u8 * 3 + row as u8 / 2);
                phrase.steps[row].instrument = Some((id & 1) as u8);
            }
            phrase.steps[3].effects[0] = Effect {
                command: Some(b'R'),
                value: 0x34,
            };
            phrase.steps[15].note = Note::Off;
            project.phrases[id] = Some(phrase);
        }
        for id in 0..4 {
            let mut chain = Chain::default();
            for row in 0..4 {
                chain.steps[row].phrase = Some(((id + row) % 6) as u8);
                chain.steps[row].transpose = if row == 2 { 12 } else { 0 };
            }
            project.chains[id] = Some(chain);
        }
        for row in 0..8 {
            for track in 0..TRACKS {
                if !(row == 5 && track % 2 == 1) {
                    project.song[row][track] = Some(((row + track) % 4) as u8);
                }
            }
        }
        project
    }
}

fn allocate<T: Copy, const N: usize>(
    slots: &mut [Option<T>; N],
    value: T,
) -> Result<ObjectId, ModelError> {
    let index = slots
        .iter()
        .position(Option::is_none)
        .ok_or(ModelError::Full)?;
    slots[index] = Some(value);
    Ok(index as ObjectId)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_lowest_and_validates_references() {
        let mut project = Project::default();
        assert_eq!(project.allocate_chain(), Ok(0));
        assert_eq!(project.allocate_chain(), Ok(1));
        project.song[0][0] = Some(1);
        assert_eq!(project.validate(), Ok(()));
        project.song[0][1] = Some(9);
        assert_eq!(project.validate(), Err(ModelError::InvalidChain));
    }

    #[test]
    fn example_has_shared_references_and_empty_cells() {
        let project = Project::example();
        assert_eq!(project.validate(), Ok(()));
        assert!(project.chain_references(0) > 1);
        assert!(project.phrase_references(0) > 1);
        assert_eq!(project.song[5][1], None);
    }
}

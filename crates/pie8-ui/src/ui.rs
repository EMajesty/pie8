use crate::{
    input::{Action, Direction},
    model::{Effect, LoopMode, ModelError, Note, ObjectId, Project, TRACKS},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Live,
    Song,
    Chain,
    Phrase,
    Instrument,
    Table,
    Groove,
    Mixer,
    Effects,
    Project,
    Settings,
    SampleBrowser,
    SampleEditor,
    Diagnostics,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cursor {
    pub row: u8,
    pub column: u8,
    pub scroll: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    pub anchor_row: u8,
    pub anchor_column: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Cell {
    #[default]
    Empty,
    Id(ObjectId),
    Signed(i8),
    Note(Note),
    Effect(Effect),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Clipboard {
    screen: Screen,
    width: u8,
    height: u8,
    cells: [[Cell; 8]; 16],
}

impl Clipboard {
    const fn empty() -> Self {
        Self {
            screen: Screen::Song,
            width: 0,
            height: 0,
            cells: [[Cell::Empty; 8]; 16],
        }
    }
}

pub struct Controller {
    pub project: Project,
    pub screen: Screen,
    pub cursors: [Cursor; 14],
    pub current_chain: ObjectId,
    pub current_phrase: ObjectId,
    pub current_instrument: ObjectId,
    pub playing: bool,
    pub selection: Option<Selection>,
    pub status: &'static str,
    clipboard: Clipboard,
    last_note: u8,
    last_instrument: ObjectId,
    last_effect: Effect,
}

impl Controller {
    #[must_use]
    pub fn example() -> Self {
        Self::new(Project::example())
    }
    #[must_use]
    pub fn new(project: Project) -> Self {
        Self {
            project,
            screen: Screen::Song,
            cursors: [Cursor::default(); 14],
            current_chain: 0,
            current_phrase: 0,
            current_instrument: 0,
            playing: false,
            selection: None,
            status: "READY",
            clipboard: Clipboard::empty(),
            last_note: 48,
            last_instrument: 0,
            last_effect: Effect {
                command: Some(b'R'),
                value: 0x34,
            },
        }
    }
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.cursors[screen_index(self.screen)]
    }
    fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[screen_index(self.screen)]
    }

    pub fn action(&mut self, action: Action) {
        self.status = "";
        match action {
            Action::Move(direction) => self.move_cursor(direction),
            Action::Navigate(direction) => self.navigate(direction),
            Action::Fine(direction) => self.adjust(direction, false),
            Action::Coarse(direction) => self.adjust(direction, true),
            Action::Edit => self.insert(),
            Action::Clear => {
                if self.selection.is_some() {
                    self.cut()
                } else {
                    self.clear()
                }
            }
            Action::Play | Action::GlobalPlay => {
                self.playing = !self.playing;
                self.status = if self.playing { "PLAYING" } else { "STOPPED" }
            }
            Action::Audition => self.status = "AUDITION (SILENT)",
            Action::BeginSelection => {
                let c = self.cursor();
                self.selection = Some(Selection {
                    anchor_row: c.row,
                    anchor_column: c.column,
                });
                self.status = "SELECT"
            }
            Action::Option => {
                if self.selection.is_some() {
                    self.copy()
                } else {
                    self.go_back()
                }
            }
            Action::Copy => self.copy(),
            Action::Cut => self.cut(),
            Action::Paste => self.paste(),
            Action::Clone => self.clone_reference(),
            Action::MutePress => self.status = "MUTE HELD",
            Action::MuteRelease => self.status = "MUTE RELEASED",
            Action::SoloPress => self.status = "SOLO HELD",
            Action::SoloRelease => self.status = "SOLO RELEASED",
            Action::ClearMutes => self.status = "MUTES CLEARED",
        }
    }

    fn move_cursor(&mut self, d: Direction) {
        let screen = self.screen;
        let c = self.cursor_mut();
        match d {
            Direction::Up => c.row = c.row.saturating_sub(1),
            Direction::Down => c.row = c.row.saturating_add(1).min(max_row(screen)),
            Direction::Left => c.column = c.column.saturating_sub(1),
            Direction::Right => c.column = c.column.saturating_add(1).min(max_column(screen)),
        }
        if screen == Screen::Song {
            c.scroll = c.row.saturating_sub(15);
        }
    }

    fn navigate(&mut self, d: Direction) {
        let target = match (self.screen, d) {
            (Screen::Song, Direction::Left) => Some(Screen::Live),
            (Screen::Song, Direction::Right) => Some(Screen::Chain),
            (Screen::Song, Direction::Down) => Some(Screen::Mixer),
            (Screen::Chain, Direction::Left) => Some(Screen::Song),
            (Screen::Chain, Direction::Right) => Some(Screen::Phrase),
            (Screen::Chain, Direction::Up) => Some(Screen::Project),
            (Screen::Chain, Direction::Down) => Some(Screen::Groove),
            (Screen::Phrase, Direction::Left) => Some(Screen::Chain),
            (Screen::Phrase, Direction::Right) => Some(Screen::Instrument),
            (Screen::Phrase, Direction::Down) => Some(Screen::Table),
            (Screen::Instrument, Direction::Left) => Some(Screen::Phrase),
            (Screen::Instrument, Direction::Down) => Some(Screen::SampleBrowser),
            (Screen::Table, Direction::Up) => Some(Screen::Phrase),
            (Screen::Table, Direction::Right) => Some(Screen::Effects),
            (Screen::Groove, Direction::Up) => Some(Screen::Chain),
            (Screen::Mixer, Direction::Up) => Some(Screen::Song),
            (Screen::Mixer, Direction::Down) => Some(Screen::Diagnostics),
            (Screen::Effects, Direction::Left) => Some(Screen::Table),
            (Screen::Project, Direction::Down) => Some(Screen::Chain),
            (Screen::Project, Direction::Right) => Some(Screen::Settings),
            (Screen::Settings, Direction::Left) => Some(Screen::Project),
            (Screen::SampleBrowser, Direction::Up) => Some(Screen::Effects),
            (Screen::SampleBrowser, Direction::Right) => Some(Screen::SampleEditor),
            (Screen::SampleEditor, Direction::Left) => Some(Screen::SampleBrowser),
            (Screen::Diagnostics, Direction::Up) => Some(Screen::Mixer),
            (Screen::Live, Direction::Right) => Some(Screen::Song),
            _ => None,
        };
        if let Some(target) = target {
            if !self.resolve_context(target) {
                self.status = "EMPTY REFERENCE";
                return;
            }
            self.screen = target;
            self.selection = None;
        } else {
            self.status = "NO VIEW"
        }
    }

    fn resolve_context(&mut self, target: Screen) -> bool {
        match (self.screen, target) {
            (Screen::Song, Screen::Chain) => {
                let c = self.cursor();
                if let Some(id) = self.project.song[c.row as usize][c.column as usize] {
                    if self
                        .project
                        .chains
                        .get(id as usize)
                        .and_then(|v| *v)
                        .is_some()
                    {
                        self.current_chain = id;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            (Screen::Chain, Screen::Phrase) => {
                let row = self.cursor().row as usize;
                if let Some(id) = self.project.chains[self.current_chain as usize]
                    .and_then(|c| c.steps[row].phrase)
                {
                    if self
                        .project
                        .phrases
                        .get(id as usize)
                        .and_then(|v| *v)
                        .is_some()
                    {
                        self.current_phrase = id;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            (Screen::Phrase, Screen::Instrument) => {
                let row = self.cursor().row as usize;
                if let Some(id) = self.project.phrases[self.current_phrase as usize]
                    .and_then(|p| p.steps[row].instrument)
                {
                    if self
                        .project
                        .instruments
                        .get(id as usize)
                        .and_then(|v| *v)
                        .is_some()
                    {
                        self.current_instrument = id;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    fn go_back(&mut self) {
        self.screen = match self.screen {
            Screen::Chain => Screen::Song,
            Screen::Phrase => Screen::Chain,
            Screen::Instrument => Screen::Phrase,
            Screen::SampleEditor => Screen::SampleBrowser,
            Screen::SampleBrowser => Screen::Instrument,
            _ => self.screen,
        };
    }

    fn insert(&mut self) {
        let c = self.cursor();
        let result: Result<(), ModelError> = match self.screen {
            Screen::Song => {
                if self.project.song[c.row as usize][c.column as usize].is_none() {
                    self.project.allocate_chain().map(|id| {
                        self.project.song[c.row as usize][c.column as usize] = Some(id);
                        self.current_chain = id
                    })
                } else {
                    Ok(())
                }
            }
            Screen::Chain => {
                let chain_id = self.current_chain as usize;
                if c.column == 0 {
                    if self.project.chains[chain_id].unwrap().steps[c.row as usize]
                        .phrase
                        .is_none()
                    {
                        self.project.allocate_phrase().map(|id| {
                            self.project.chains[chain_id].as_mut().unwrap().steps[c.row as usize]
                                .phrase = Some(id);
                            self.current_phrase = id
                        })
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            }
            Screen::Phrase => {
                let step = &mut self.project.phrases[self.current_phrase as usize]
                    .as_mut()
                    .unwrap()
                    .steps[c.row as usize];
                match c.column {
                    0 => {
                        if step.note == Note::Empty {
                            step.note = Note::Pitch(self.last_note)
                        }
                    }
                    1 => {
                        if step.instrument.is_none() {
                            step.instrument = Some(self.last_instrument)
                        }
                    }
                    2 => {
                        if step.effects[0].command.is_none() {
                            step.effects[0] = self.last_effect
                        }
                    }
                    3 => {
                        if step.effects[1].command.is_none() {
                            step.effects[1] = self.last_effect
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            _ => Ok(()),
        };
        self.status = if result.is_ok() {
            "INSERT"
        } else {
            "OBJECT POOL FULL"
        };
    }

    fn clear(&mut self) {
        let c = self.cursor();
        match self.screen {
            Screen::Song => self.project.song[c.row as usize][c.column as usize] = None,
            Screen::Chain => {
                let step = &mut self.project.chains[self.current_chain as usize]
                    .as_mut()
                    .unwrap()
                    .steps[c.row as usize];
                if c.column == 0 {
                    step.phrase = None
                } else {
                    step.transpose = 0
                }
            }
            Screen::Phrase => {
                let step = &mut self.project.phrases[self.current_phrase as usize]
                    .as_mut()
                    .unwrap()
                    .steps[c.row as usize];
                match c.column {
                    0 => step.note = Note::Empty,
                    1 => step.instrument = None,
                    2 => step.effects[0] = Effect::default(),
                    3 => step.effects[1] = Effect::default(),
                    _ => {}
                }
            }
            _ => {}
        }
        self.status = "CLEARED";
    }

    fn adjust(&mut self, d: Direction, coarse: bool) {
        let delta = match d {
            Direction::Left | Direction::Down => -(if coarse { 12 } else { 1 }),
            _ => {
                if coarse {
                    12
                } else {
                    1
                }
            }
        };
        let c = self.cursor();
        match self.screen {
            Screen::Song => {
                if let Some(id) = self.project.song[c.row as usize][c.column as usize] {
                    let candidate = id.saturating_add_signed(delta);
                    if self
                        .project
                        .chains
                        .get(candidate as usize)
                        .and_then(|v| *v)
                        .is_some()
                    {
                        self.project.song[c.row as usize][c.column as usize] = Some(candidate);
                    }
                }
            }
            Screen::Phrase => {
                let step = &mut self.project.phrases[self.current_phrase as usize]
                    .as_mut()
                    .unwrap()
                    .steps[c.row as usize];
                match c.column {
                    0 => {
                        if step.note == Note::Empty {
                            step.note = Note::Pitch(self.last_note)
                        }
                        step.note = step.note.transpose(delta);
                        if let Note::Pitch(n) = step.note {
                            self.last_note = n
                        }
                    }
                    1 => {
                        if let Some(id) = step.instrument {
                            let candidate = id.saturating_add_signed(delta);
                            if self
                                .project
                                .instruments
                                .get(candidate as usize)
                                .and_then(|v| *v)
                                .is_some()
                            {
                                step.instrument = Some(candidate);
                            }
                        }
                    }
                    2 | 3 => {
                        let fx = &mut step.effects[c.column as usize - 2];
                        fx.value = fx.value.saturating_add_signed(delta)
                    }
                    _ => {}
                }
            }
            Screen::Chain => {
                let step = &mut self.project.chains[self.current_chain as usize]
                    .as_mut()
                    .unwrap()
                    .steps[c.row as usize];
                if c.column == 0 {
                    if let Some(id) = step.phrase {
                        let candidate = id.saturating_add_signed(delta);
                        if self
                            .project
                            .phrases
                            .get(candidate as usize)
                            .and_then(|v| *v)
                            .is_some()
                        {
                            step.phrase = Some(candidate);
                        }
                    }
                } else {
                    step.transpose = step.transpose.saturating_add(delta).clamp(-96, 96)
                }
            }
            Screen::Instrument => self.adjust_instrument(c.row, delta),
            _ => {}
        }
        self.status = if coarse { "COARSE" } else { "FINE" };
    }

    fn adjust_instrument(&mut self, row: u8, delta: i8) {
        let s = &mut self.project.instruments[self.current_instrument as usize]
            .as_mut()
            .unwrap()
            .sampler;
        match row {
            1 => s.gain = (s.gain + delta as f32 / 100.0).clamp(0.0, 4.0),
            2 => s.pan = (s.pan + delta as f32 / 100.0).clamp(-1.0, 1.0),
            3 => s.semitones = s.semitones.saturating_add(delta).clamp(-96, 96),
            4 => s.cents = s.cents.saturating_add(delta as i16).clamp(-100, 100),
            5 => {
                s.direction = match s.direction {
                    crate::model::Direction::Forward => crate::model::Direction::Reverse,
                    _ => crate::model::Direction::Forward,
                }
            }
            6 => {
                s.loop_mode = match s.loop_mode {
                    LoopMode::OneShot => LoopMode::Forward,
                    LoopMode::Forward => LoopMode::PingPong,
                    LoopMode::PingPong => LoopMode::OneShot,
                }
            }
            7 => s.start_frame = s.start_frame.saturating_add_signed(delta as i32),
            8 => s.end_frame = s.end_frame.saturating_add_signed(delta as i32),
            9 => s.attack_ms = s.attack_ms.saturating_add_signed(delta as i16),
            10 => s.decay_ms = s.decay_ms.saturating_add_signed(delta as i16),
            11 => s.sustain = s.sustain.saturating_add_signed(delta),
            12 => s.release_ms = s.release_ms.saturating_add_signed(delta as i16),
            _ => {}
        }
    }

    fn clone_reference(&mut self) {
        let c = self.cursor();
        let result = match self.screen {
            Screen::Song => self.project.song[c.row as usize][c.column as usize]
                .ok_or(ModelError::InvalidChain)
                .and_then(|old| self.project.clone_chain(old))
                .map(|id| self.project.song[c.row as usize][c.column as usize] = Some(id)),
            Screen::Chain => self.project.chains[self.current_chain as usize]
                .unwrap()
                .steps[c.row as usize]
                .phrase
                .ok_or(ModelError::InvalidPhrase)
                .and_then(|old| self.project.clone_phrase(old))
                .map(|id| {
                    self.project.chains[self.current_chain as usize]
                        .as_mut()
                        .unwrap()
                        .steps[c.row as usize]
                        .phrase = Some(id)
                }),
            Screen::Phrase if c.column == 1 => {
                let old = self.project.phrases[self.current_phrase as usize]
                    .unwrap()
                    .steps[c.row as usize]
                    .instrument
                    .ok_or(ModelError::InvalidInstrument);
                old.and_then(|id| {
                    let value = self.project.instruments[id as usize].unwrap();
                    let new = self.project.allocate_instrument()?;
                    self.project.instruments[new as usize] = Some(value);
                    self.project.phrases[self.current_phrase as usize]
                        .as_mut()
                        .unwrap()
                        .steps[c.row as usize]
                        .instrument = Some(new);
                    Ok(())
                })
            }
            _ => Err(ModelError::InvalidChain),
        };
        self.status = if result.is_ok() {
            "CLONED"
        } else {
            "CLONE FAILED"
        };
    }

    fn cell(&self, row: u8, column: u8) -> Cell {
        match self.screen {
            Screen::Song => {
                self.project.song[row as usize][column as usize].map_or(Cell::Empty, Cell::Id)
            }
            Screen::Chain => {
                let s = self.project.chains[self.current_chain as usize]
                    .unwrap()
                    .steps[row as usize];
                if column == 0 {
                    s.phrase.map_or(Cell::Empty, Cell::Id)
                } else {
                    Cell::Signed(s.transpose)
                }
            }
            Screen::Phrase => {
                let s = self.project.phrases[self.current_phrase as usize]
                    .unwrap()
                    .steps[row as usize];
                match column {
                    0 => Cell::Note(s.note),
                    1 => s.instrument.map_or(Cell::Empty, Cell::Id),
                    2 => Cell::Effect(s.effects[0]),
                    3 => Cell::Effect(s.effects[1]),
                    _ => Cell::Empty,
                }
            }
            _ => Cell::Empty,
        }
    }
    fn set_cell(&mut self, row: u8, column: u8, value: Cell) {
        match self.screen {
            Screen::Song => {
                self.project.song[row as usize][column as usize] = if let Cell::Id(v) = value {
                    Some(v)
                } else {
                    None
                }
            }
            Screen::Chain => {
                let s = &mut self.project.chains[self.current_chain as usize]
                    .as_mut()
                    .unwrap()
                    .steps[row as usize];
                if column == 0 {
                    s.phrase = if let Cell::Id(v) = value {
                        Some(v)
                    } else {
                        None
                    }
                } else if let Cell::Signed(v) = value {
                    s.transpose = v
                }
            }
            Screen::Phrase => {
                let s = &mut self.project.phrases[self.current_phrase as usize]
                    .as_mut()
                    .unwrap()
                    .steps[row as usize];
                match (column, value) {
                    (0, Cell::Note(v)) => s.note = v,
                    (1, Cell::Id(v)) => s.instrument = Some(v),
                    (1, Cell::Empty) => s.instrument = None,
                    (2, Cell::Effect(v)) => s.effects[0] = v,
                    (3, Cell::Effect(v)) => s.effects[1] = v,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn selection_bounds(&self) -> (u8, u8, u8, u8) {
        let c = self.cursor();
        let a = self.selection.unwrap();
        (
            a.anchor_row.min(c.row),
            a.anchor_row.max(c.row),
            a.anchor_column.min(c.column),
            a.anchor_column.max(c.column),
        )
    }
    fn copy(&mut self) {
        if self.selection.is_none() {
            return;
        }
        let (r0, r1, c0, c1) = self.selection_bounds();
        let mut clip = Clipboard::empty();
        clip.screen = self.screen;
        clip.height = (r1 - r0 + 1).min(16);
        clip.width = (c1 - c0 + 1).min(8);
        for r in 0..clip.height {
            for c in 0..clip.width {
                clip.cells[r as usize][c as usize] = self.cell(r0 + r, c0 + c)
            }
        }
        self.clipboard = clip;
        self.selection = None;
        self.status = "COPIED";
    }
    fn cut(&mut self) {
        if self.selection.is_none() {
            return;
        }
        let (r0, r1, c0, c1) = self.selection_bounds();
        self.copy();
        for r in r0..=r1 {
            for c in c0..=c1 {
                self.set_cell(r, c, Cell::Empty)
            }
        }
        self.status = "CUT";
    }
    fn paste(&mut self) {
        if self.clipboard.width == 0 || self.clipboard.screen != self.screen {
            self.status = "NO COMPATIBLE COPY";
            return;
        }
        let at = self.cursor();
        for r in 0..self.clipboard.height {
            for c in 0..self.clipboard.width {
                let row = at.row.saturating_add(r);
                let col = at.column.saturating_add(c);
                if row <= max_row(self.screen) && col <= max_column(self.screen) {
                    self.set_cell(row, col, self.clipboard.cells[r as usize][c as usize])
                }
            }
        }
        self.status = "PASTED";
    }
}

const fn screen_index(s: Screen) -> usize {
    s as usize
}
const fn max_row(s: Screen) -> u8 {
    match s {
        Screen::Song => 63,
        Screen::Chain | Screen::Phrase | Screen::Table | Screen::Groove => 15,
        Screen::Instrument => 12,
        _ => 15,
    }
}
const fn max_column(s: Screen) -> u8 {
    match s {
        Screen::Song => TRACKS as u8 - 1,
        Screen::Chain => 1,
        Screen::Phrase => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_restores_parent_context() {
        let mut c = Controller::example();
        c.cursors[screen_index(Screen::Song)] = Cursor {
            row: 1,
            column: 2,
            scroll: 0,
        };
        let id = c.project.song[1][2].unwrap();
        c.action(Action::Navigate(Direction::Right));
        assert_eq!(c.screen, Screen::Chain);
        assert_eq!(c.current_chain, id);
        c.cursors[screen_index(Screen::Chain)].row = 2;
        c.action(Action::Navigate(Direction::Right));
        assert_eq!(c.screen, Screen::Phrase);
        c.action(Action::Option);
        assert_eq!(c.screen, Screen::Chain);
        assert_eq!(c.cursor().row, 2);
        c.action(Action::Option);
        assert_eq!(
            c.cursor(),
            Cursor {
                row: 1,
                column: 2,
                scroll: 0
            }
        );
    }
    #[test]
    fn insert_note_adjust_and_clear() {
        let mut c = Controller::example();
        c.screen = Screen::Phrase;
        c.current_phrase = 0;
        c.cursors[screen_index(Screen::Phrase)].row = 1;
        c.action(Action::Edit);
        assert_eq!(c.project.phrases[0].unwrap().steps[1].note, Note::Pitch(48));
        c.action(Action::Fine(Direction::Right));
        assert_eq!(c.project.phrases[0].unwrap().steps[1].note, Note::Pitch(49));
        c.action(Action::Clear);
        assert_eq!(c.project.phrases[0].unwrap().steps[1].note, Note::Empty);
    }
    #[test]
    fn creates_and_clones_lowest_object() {
        let mut c = Controller::new(Project::default());
        c.action(Action::Edit);
        assert_eq!(c.project.song[0][0], Some(0));
        c.action(Action::Clone);
        assert_eq!(c.project.song[0][0], Some(1));
        assert_eq!(c.project.chain_references(0), 0);
    }
    #[test]
    fn clipboard_copies_and_cuts() {
        let mut c = Controller::example();
        let original = c.project.song[0][0];
        c.selection = Some(Selection {
            anchor_row: 0,
            anchor_column: 0,
        });
        c.action(Action::Option);
        c.cursors[screen_index(Screen::Song)].row = 10;
        c.action(Action::Paste);
        assert_eq!(c.project.song[10][0], original);
        c.selection = Some(Selection {
            anchor_row: 10,
            anchor_column: 0,
        });
        c.action(Action::Clear);
        assert_eq!(c.project.song[10][0], None);
    }

    #[test]
    fn invalid_reference_cannot_be_entered() {
        let mut project = Project::default();
        project.song[0][0] = Some(63);
        let mut controller = Controller::new(project);
        controller.action(Action::Navigate(Direction::Right));
        assert_eq!(controller.screen, Screen::Song);
        assert_eq!(controller.status, "EMPTY REFERENCE");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    Shift,
    Edit,
    Option,
    Play,
}

impl Button {
    const fn bit(self) -> u8 {
        1 << self as u8
    }
    const fn direction(self) -> Option<Direction> {
        match self {
            Self::Up => Some(Direction::Up),
            Self::Down => Some(Direction::Down),
            Self::Left => Some(Direction::Left),
            Self::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    Press,
    Release,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ButtonEvent {
    pub button: Button,
    pub kind: EventKind,
    pub timestamp_us: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Move(Direction),
    Navigate(Direction),
    Fine(Direction),
    Coarse(Direction),
    Edit,
    Clear,
    Play,
    GlobalPlay,
    Audition,
    Option,
    BeginSelection,
    Copy,
    Cut,
    Paste,
    Clone,
    MutePress,
    MuteRelease,
    SoloPress,
    SoloRelease,
    ClearMutes,
}

pub const REPEAT_DELAY_US: u64 = 350_000;
pub const REPEAT_US: u64 = 70_000;
pub const FAST_REPEAT_AFTER_US: u64 = 1_500_000;
pub const FAST_REPEAT_US: u64 = 35_000;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    held: u8,
    pressed_at: [u64; 8],
    next_repeat: [u64; 4],
    selection_pending: bool,
    mute_active: bool,
    solo_active: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            held: 0,
            pressed_at: [0; 8],
            next_repeat: [u64::MAX; 4],
            selection_pending: false,
            mute_active: false,
            solo_active: false,
        }
    }
}

impl Input {
    #[must_use]
    pub const fn is_held(&self, button: Button) -> bool {
        self.held & button.bit() != 0
    }

    pub fn event(&mut self, event: ButtonEvent) -> Option<Action> {
        match event.kind {
            EventKind::Press => self.press(event.button, event.timestamp_us),
            EventKind::Release => self.release(event.button),
        }
    }

    fn press(&mut self, button: Button, now: u64) -> Option<Action> {
        if self.is_held(button) {
            return None;
        }
        let before = self.held;
        self.held |= button.bit();
        self.pressed_at[button as usize] = now;
        if let Some(direction) = button.direction() {
            self.next_repeat[direction_index(direction)] = now.saturating_add(REPEAT_DELAY_US);
            if before & Button::Edit.bit() != 0 {
                return Some(if matches!(direction, Direction::Left | Direction::Right) {
                    Action::Fine(direction)
                } else {
                    Action::Coarse(direction)
                });
            }
            if before & Button::Shift.bit() != 0 {
                return Some(Action::Navigate(direction));
            }
            return Some(Action::Move(direction));
        }
        match button {
            Button::Option if before & Button::Edit.bit() != 0 => Some(Action::Clear),
            Button::Option if before & Button::Shift.bit() != 0 => {
                self.selection_pending = true;
                None
            }
            Button::Shift if before & Button::Option.bit() != 0 => {
                self.mute_active = true;
                Some(Action::MutePress)
            }
            Button::Edit
                if before & (Button::Shift.bit() | Button::Option.bit())
                    == (Button::Shift.bit() | Button::Option.bit()) =>
            {
                self.selection_pending = false;
                Some(Action::Clone)
            }
            Button::Edit if before & Button::Option.bit() != 0 => Some(Action::Clear),
            Button::Edit if before & Button::Shift.bit() != 0 => Some(Action::Paste),
            Button::Edit => Some(Action::Edit),
            Button::Option => None,
            Button::Play
                if before & (Button::Shift.bit() | Button::Option.bit())
                    == (Button::Shift.bit() | Button::Option.bit()) =>
            {
                Some(Action::ClearMutes)
            }
            Button::Play if before & Button::Shift.bit() != 0 => Some(Action::GlobalPlay),
            Button::Play if before & Button::Option.bit() != 0 => {
                self.solo_active = true;
                Some(Action::SoloPress)
            }
            Button::Play if before & Button::Edit.bit() != 0 => Some(Action::Audition),
            Button::Play => Some(Action::Play),
            _ => None,
        }
    }

    fn release(&mut self, button: Button) -> Option<Action> {
        if !self.is_held(button) {
            return None;
        }
        self.held &= !button.bit();
        if let Some(direction) = button.direction() {
            self.next_repeat[direction_index(direction)] = u64::MAX;
        }
        if button == Button::Option && self.selection_pending {
            self.selection_pending = false;
            return Some(Action::BeginSelection);
        }
        if self.mute_active && (button == Button::Option || button == Button::Shift) {
            self.mute_active = false;
            return Some(Action::MuteRelease);
        }
        if self.solo_active && (button == Button::Option || button == Button::Play) {
            self.solo_active = false;
            return Some(Action::SoloRelease);
        }
        if button == Button::Option && self.held == 0 {
            return Some(Action::Option);
        }
        None
    }

    pub fn tick(&mut self, now: u64) -> Option<Action> {
        for (index, direction) in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .enumerate()
        {
            let button = [Button::Up, Button::Down, Button::Left, Button::Right][index];
            if self.is_held(button) && now >= self.next_repeat[index] {
                let held_for = now.saturating_sub(self.pressed_at[button as usize]);
                let interval = if held_for >= FAST_REPEAT_AFTER_US {
                    FAST_REPEAT_US
                } else {
                    REPEAT_US
                };
                self.next_repeat[index] = now.saturating_add(interval);
                if self.is_held(Button::Edit) {
                    return Some(if matches!(direction, Direction::Left | Direction::Right) {
                        Action::Fine(direction)
                    } else {
                        Action::Coarse(direction)
                    });
                }
                if self.is_held(Button::Shift) {
                    return None;
                }
                return Some(Action::Move(direction));
            }
        }
        None
    }
}

const fn direction_index(direction: Direction) -> usize {
    match direction {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn press(button: Button, t: u64) -> ButtonEvent {
        ButtonEvent {
            button,
            kind: EventKind::Press,
            timestamp_us: t,
        }
    }
    fn release(button: Button, t: u64) -> ButtonEvent {
        ButtonEvent {
            button,
            kind: EventKind::Release,
            timestamp_us: t,
        }
    }

    #[test]
    fn held_modifier_has_no_chord_window() {
        let mut i = Input::default();
        assert_eq!(i.event(press(Button::Shift, 0)), None);
        assert_eq!(
            i.event(press(Button::Right, 2_000_000)),
            Some(Action::Navigate(Direction::Right))
        );
    }
    #[test]
    fn order_resolves_selection_mute_and_clone() {
        let mut i = Input::default();
        i.event(press(Button::Shift, 0));
        i.event(press(Button::Option, 10));
        assert_eq!(
            i.event(release(Button::Option, 20)),
            Some(Action::BeginSelection)
        );
        let mut i = Input::default();
        i.event(press(Button::Option, 0));
        assert_eq!(i.event(press(Button::Shift, 10)), Some(Action::MutePress));
        let mut i = Input::default();
        i.event(press(Button::Shift, 0));
        i.event(press(Button::Option, 10));
        assert_eq!(i.event(press(Button::Edit, 20)), Some(Action::Clone));
    }
    #[test]
    fn repeat_timing_is_portable() {
        let mut i = Input::default();
        assert_eq!(
            i.event(press(Button::Down, 0)),
            Some(Action::Move(Direction::Down))
        );
        assert_eq!(i.tick(REPEAT_DELAY_US - 1), None);
        assert_eq!(i.tick(REPEAT_DELAY_US), Some(Action::Move(Direction::Down)));
        assert_eq!(
            i.tick(REPEAT_DELAY_US + REPEAT_US),
            Some(Action::Move(Direction::Down))
        );
    }
    #[test]
    fn simultaneous_duplicate_edges_are_ignored() {
        let mut i = Input::default();
        assert!(i.event(press(Button::Left, 1)).is_some());
        assert_eq!(i.event(press(Button::Left, 2)), None);
        assert_eq!(i.event(release(Button::Left, 3)), None);
    }

    #[test]
    fn longest_play_chord_has_precedence() {
        let mut input = Input::default();
        input.event(press(Button::Shift, 0));
        input.event(press(Button::Option, 10));
        assert_eq!(
            input.event(press(Button::Play, 20)),
            Some(Action::ClearMutes)
        );
    }
}

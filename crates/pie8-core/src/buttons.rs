pub const DEFAULT_DEBOUNCE_MS: u64 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonEventKind {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ButtonEvent {
    pub index: usize,
    pub kind: ButtonEventKind,
    pub timestamp_ms: u64,
    /// Duration of the completed press. Zero for press events.
    pub duration_ms: u64,
}

#[derive(Clone, Copy)]
struct Button {
    stable: bool,
    candidate: bool,
    candidate_since: u64,
    pressed_at: u64,
}

impl Button {
    const NEW: Self = Self {
        stable: false,
        candidate: false,
        candidate_since: 0,
        pressed_at: 0,
    };
}

/// Allocation-free debouncer. Input values are logical pressed states, after
/// active-low conversion by the caller.
pub struct Buttons<const N: usize> {
    buttons: [Button; N],
    debounce_ms: u64,
}

impl<const N: usize> Buttons<N> {
    #[must_use]
    pub const fn new(debounce_ms: u64) -> Self {
        Self {
            buttons: [Button::NEW; N],
            debounce_ms,
        }
    }

    #[must_use]
    pub fn update(&mut self, pressed: [bool; N], now_ms: u64) -> [Option<ButtonEvent>; N] {
        let mut events = [None; N];
        for (index, (button, &sample)) in self.buttons.iter_mut().zip(&pressed).enumerate() {
            if sample != button.candidate {
                button.candidate = sample;
                button.candidate_since = now_ms;
            }
            if button.candidate != button.stable
                && now_ms.wrapping_sub(button.candidate_since) >= self.debounce_ms
            {
                button.stable = button.candidate;
                let duration_ms = if button.stable {
                    button.pressed_at = now_ms;
                    0
                } else {
                    now_ms.wrapping_sub(button.pressed_at)
                };
                events[index] = Some(ButtonEvent {
                    index,
                    kind: if button.stable {
                        ButtonEventKind::Pressed
                    } else {
                        ButtonEventKind::Released
                    },
                    timestamp_ms: now_ms,
                    duration_ms,
                });
            }
        }
        events
    }

    #[must_use]
    pub fn is_pressed(&self, index: usize) -> bool {
        self.buttons[index].stable
    }

    #[must_use]
    pub fn press_duration_ms(&self, index: usize, now_ms: u64) -> u64 {
        let button = &self.buttons[index];
        if button.stable {
            now_ms.wrapping_sub(button.pressed_at)
        } else {
            0
        }
    }
}

impl<const N: usize> Default for Buttons<N> {
    fn default() -> Self {
        Self::new(DEFAULT_DEBOUNCE_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bounce_and_emits_press_and_release() {
        let mut buttons = Buttons::<1>::default();
        assert_eq!(buttons.update([true], 1), [None]);
        assert_eq!(buttons.update([false], 5), [None]);
        assert_eq!(buttons.update([true], 7), [None]);
        let press = buttons.update([true], 17)[0].unwrap();
        assert_eq!(press.kind, ButtonEventKind::Pressed);
        assert!(buttons.is_pressed(0));
        assert_eq!(buttons.press_duration_ms(0, 27), 10);
        assert_eq!(buttons.update([false], 30), [None]);
        let release = buttons.update([false], 40)[0].unwrap();
        assert_eq!(release.kind, ButtonEventKind::Released);
        assert_eq!(release.duration_ms, 23);
    }

    #[test]
    fn simultaneous_buttons_have_independent_events() {
        let mut buttons = Buttons::<3>::new(10);
        let _ = buttons.update([true, false, true], 100);
        let events = buttons.update([true, false, true], 110);
        assert!(events[0].is_some());
        assert!(events[1].is_none());
        assert!(events[2].is_some());
    }

    #[test]
    fn timestamps_use_wrapping_elapsed_time() {
        let mut buttons = Buttons::<1>::new(5);
        let _ = buttons.update([true], u64::MAX - 2);
        let event = buttons.update([true], 3)[0].unwrap();
        assert_eq!(event.timestamp_ms, 3);
        assert_eq!(event.kind, ButtonEventKind::Pressed);
    }
}

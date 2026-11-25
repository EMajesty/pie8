use pie8_core::graphics::{Rect, Rgb565, Surface};

use crate::{
    model::{Direction as PlayDirection, LoopMode, Note, TRACKS},
    ui::{Controller, Screen},
};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: Rgb565,
    pub panel: Rgb565,
    pub text: Rgb565,
    pub muted: Rgb565,
    pub accent: Rgb565,
    pub secondary: Rgb565,
    pub warning: Rgb565,
    pub cursor_bg: Rgb565,
    pub cursor_fg: Rgb565,
    pub selection: Rgb565,
    pub playhead: Rgb565,
    pub beat_major: Rgb565,
    pub beat_minor: Rgb565,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Rgb565::new(7, 16, 24),
            panel: Rgb565::new(16, 31, 43),
            text: Rgb565::new(216, 222, 213),
            muted: Rgb565::new(98, 116, 131),
            accent: Rgb565::new(57, 214, 207),
            secondary: Rgb565::new(229, 116, 199),
            warning: Rgb565::new(240, 189, 83),
            cursor_bg: Rgb565::new(216, 243, 232),
            cursor_fg: Rgb565::new(7, 16, 24),
            selection: Rgb565::new(23, 63, 99),
            playhead: Rgb565::new(142, 227, 91),
            beat_major: Rgb565::new(19, 43, 56),
            beat_minor: Rgb565::new(12, 25, 35),
        }
    }
}

pub fn render(controller: &Controller, pixels: &mut [u16], palette: &Palette) {
    let mut s = Surface::new(pixels, WIDTH as u32, HEIGHT as u32);
    s.clear(palette.background);
    header(&mut s, controller, palette);
    match controller.screen {
        Screen::Song => song(&mut s, controller, palette),
        Screen::Chain => chain(&mut s, controller, palette),
        Screen::Phrase => phrase(&mut s, controller, palette),
        Screen::Instrument => instrument(&mut s, controller, palette),
        other => placeholder(&mut s, other, palette),
    }
    status(&mut s, controller, palette);
    minimap(&mut s, controller.screen, palette);
}

fn header(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    s.fill_rect(
        Rect {
            x: 4,
            y: 4,
            width: 312,
            height: 16,
        },
        p.panel,
    );
    s.text(8, 8, screen_name(c.screen), p.accent);
    let mut id = [b' '; 4];
    let label = match c.screen {
        Screen::Chain => {
            hex(c.current_chain, &mut id[0..2]);
            &id[..2]
        }
        Screen::Phrase => {
            hex(c.current_phrase, &mut id[0..2]);
            &id[..2]
        }
        Screen::Instrument => {
            hex(c.current_instrument, &mut id[0..2]);
            &id[..2]
        }
        _ => &id[..0],
    };
    text_bytes(s, 132, 8, label, p.secondary);
    s.text(
        240,
        8,
        if c.playing { "PLAY" } else { "STOP" },
        if c.playing { p.playhead } else { p.muted },
    );
}
fn status(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    s.fill_rect(
        Rect {
            x: 4,
            y: 206,
            width: 312,
            height: 18,
        },
        p.panel,
    );
    s.text(8, 211, c.status, p.muted);
}
fn row_bg(s: &mut Surface<'_>, row: usize, p: &Palette) {
    s.fill_rect(
        Rect {
            x: 4,
            y: 31 + row as i32 * 10,
            width: 312,
            height: 10,
        },
        if row % 4 == 0 {
            p.beat_major
        } else if row % 2 == 0 {
            p.beat_minor
        } else {
            p.background
        },
    );
}
fn focus(s: &mut Surface<'_>, x: i32, y: i32, w: u32, active: bool, selected: bool, p: &Palette) {
    if selected {
        s.fill_rect(
            Rect {
                x,
                y,
                width: w,
                height: 9,
            },
            p.selection,
        )
    }
    if active {
        s.fill_rect(
            Rect {
                x,
                y,
                width: w,
                height: 9,
            },
            p.cursor_bg,
        )
    }
}
fn selected(c: &Controller, row: u8, col: u8) -> bool {
    let Some(a) = c.selection else { return false };
    let cur = c.cursor();
    row >= a.anchor_row.min(cur.row)
        && row <= a.anchor_row.max(cur.row)
        && col >= a.anchor_column.min(cur.column)
        && col <= a.anchor_column.max(cur.column)
}

fn song(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    s.text(8, 23, "#", p.muted);
    for track in 0..TRACKS {
        let mut b = [b'T', b'0'];
        b[1] = digit(track as u8);
        text_bytes(s, 28 + track as i32 * 35, 23, &b, p.muted)
    }
    let cur = c.cursor();
    let start = cur.scroll as usize;
    for visible in 0..16 {
        let row = start + visible;
        row_bg(s, visible, p);
        if c.playing && cur.row as usize == row {
            s.fill_rect(
                Rect {
                    x: 4,
                    y: 31 + visible as i32 * 10,
                    width: 2,
                    height: 9,
                },
                p.playhead,
            );
        }
        let mut rb = [0; 2];
        hex(row as u8, &mut rb);
        text_bytes(s, 8, 34 + visible as i32 * 10, &rb, p.muted);
        for track in 0..TRACKS {
            let x = 27 + track as i32 * 35;
            focus(
                s,
                x,
                31 + visible as i32 * 10,
                31,
                cur.row as usize == row && cur.column as usize == track,
                selected(c, row as u8, track as u8),
                p,
            );
            let active = cur.row as usize == row && cur.column as usize == track;
            match c.project.song[row][track] {
                Some(id) => {
                    let mut b = [0; 3];
                    hex(id, &mut b[..2]);
                    b[2] = if c.project.chain_references(id) > 1 {
                        b'*'
                    } else {
                        b' '
                    };
                    text_bytes(
                        s,
                        x + 2,
                        34 + visible as i32 * 10,
                        &b,
                        if active {
                            p.cursor_fg
                        } else if b[2] == b'*' {
                            p.warning
                        } else {
                            p.text
                        },
                    )
                }
                None => s.text(
                    x + 2,
                    34 + visible as i32 * 10,
                    "--",
                    if active { p.cursor_fg } else { p.muted },
                ),
            }
        }
    }
}

fn chain(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    s.text(8, 23, "#", p.muted);
    s.text(32, 23, "PHRASE", p.muted);
    s.text(126, 23, "TRANSPOSE", p.muted);
    let cur = c.cursor();
    let chain = c.project.chains[c.current_chain as usize].unwrap();
    for row in 0..16 {
        row_bg(s, row, p);
        if c.playing && cur.row as usize == row {
            s.fill_rect(
                Rect {
                    x: 4,
                    y: 31 + row as i32 * 10,
                    width: 2,
                    height: 9,
                },
                p.playhead,
            );
        }
        let y = 34 + row as i32 * 10;
        let r = [digit(row as u8)];
        text_bytes(s, 8, y, &r, p.muted);
        focus(
            s,
            30,
            y - 3,
            82,
            cur.row as usize == row && cur.column == 0,
            selected(c, row as u8, 0),
            p,
        );
        focus(
            s,
            124,
            y - 3,
            88,
            cur.row as usize == row && cur.column == 1,
            selected(c, row as u8, 1),
            p,
        );
        let step = chain.steps[row];
        if let Some(id) = step.phrase {
            let mut b = [0; 3];
            hex(id, &mut b[..2]);
            b[2] = if c.project.phrase_references(id) > 1 {
                b'*'
            } else {
                b' '
            };
            text_bytes(
                s,
                34,
                y,
                &b,
                if cur.row as usize == row && cur.column == 0 {
                    p.cursor_fg
                } else if b[2] == b'*' {
                    p.warning
                } else {
                    p.text
                },
            )
        } else {
            s.text(
                34,
                y,
                "--",
                if cur.row as usize == row && cur.column == 0 {
                    p.cursor_fg
                } else {
                    p.muted
                },
            )
        }
        signed(
            s,
            130,
            y,
            step.transpose,
            if cur.row as usize == row && cur.column == 1 {
                p.cursor_fg
            } else {
                p.text
            },
        );
    }
}

fn phrase(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    for (x, h) in [
        (8, "#"),
        (27, "NOTE"),
        (73, "IN"),
        (105, "FX1"),
        (171, "FX2"),
    ] {
        s.text(x, 23, h, p.muted)
    }
    let cur = c.cursor();
    let phrase = c.project.phrases[c.current_phrase as usize].unwrap();
    let xs = [25, 71, 103, 169];
    let widths = [42, 28, 62, 62];
    for row in 0..16 {
        row_bg(s, row, p);
        if c.playing && cur.row as usize == row {
            s.fill_rect(
                Rect {
                    x: 4,
                    y: 31 + row as i32 * 10,
                    width: 2,
                    height: 9,
                },
                p.playhead,
            );
        }
        let y = 34 + row as i32 * 10;
        text_bytes(s, 8, y, &[digit(row as u8)], p.muted);
        for col in 0..4 {
            focus(
                s,
                xs[col],
                y - 3,
                widths[col],
                cur.row as usize == row && cur.column as usize == col,
                selected(c, row as u8, col as u8),
                p,
            )
        }
        let step = phrase.steps[row];
        note(
            s,
            29,
            y,
            step.note,
            if cur.row as usize == row && cur.column == 0 {
                p.cursor_fg
            } else {
                p.text
            },
        );
        if let Some(id) = step.instrument {
            let mut b = [0; 2];
            hex(id, &mut b);
            text_bytes(
                s,
                75,
                y,
                &b,
                if cur.row as usize == row && cur.column == 1 {
                    p.cursor_fg
                } else {
                    p.secondary
                },
            )
        } else {
            s.text(
                75,
                y,
                "--",
                if cur.row as usize == row && cur.column == 1 {
                    p.cursor_fg
                } else {
                    p.muted
                },
            )
        }
        effect(
            s,
            107,
            y,
            step.effects[0],
            if cur.row as usize == row && cur.column == 2 {
                p.cursor_fg
            } else {
                p.text
            },
        );
        effect(
            s,
            173,
            y,
            step.effects[1],
            if cur.row as usize == row && cur.column == 3 {
                p.cursor_fg
            } else {
                p.text
            },
        );
    }
}

fn instrument(s: &mut Surface<'_>, c: &Controller, p: &Palette) {
    let Some(inst) = c.project.instruments[c.current_instrument as usize] else {
        return;
    };
    let v = &inst.sampler;
    let labels = [
        "SAMPLE",
        "GAIN",
        "PAN",
        "PITCH",
        "CENTS",
        "DIRECTION",
        "LOOP MODE",
        "START",
        "END",
        "ATTACK MS",
        "DECAY MS",
        "SUSTAIN",
        "RELEASE MS",
    ];
    for (row, label) in labels.into_iter().enumerate() {
        let y = 28 + row as i32 * 13;
        s.fill_rect(
            Rect {
                x: 6,
                y: y - 3,
                width: 308,
                height: 12,
            },
            if row % 2 == 0 {
                p.beat_major
            } else {
                p.background
            },
        );
        focus(s, 120, y - 3, 150, c.cursor().row as usize == row, false, p);
        s.text(10, y, label, p.muted);
        let color = if c.cursor().row as usize == row {
            p.cursor_fg
        } else {
            p.text
        };
        match row {
            0 => {
                if let Some(id) = v.sample {
                    let mut b = [0; 4];
                    hex16(id, &mut b);
                    text_bytes(s, 124, y, &b, color)
                } else {
                    s.text(124, y, "----", color)
                }
            }
            1 => decimal(s, 124, y, (v.gain * 100.0) as i32, color),
            2 => decimal(s, 124, y, (v.pan * 100.0) as i32, color),
            3 => signed(s, 124, y, v.semitones, color),
            4 => decimal(s, 124, y, v.cents as i32, color),
            5 => s.text(
                124,
                y,
                if v.direction == PlayDirection::Forward {
                    "FORWARD"
                } else {
                    "REVERSE"
                },
                color,
            ),
            6 => s.text(
                124,
                y,
                match v.loop_mode {
                    LoopMode::OneShot => "ONE SHOT",
                    LoopMode::Forward => "FORWARD",
                    LoopMode::PingPong => "PINGPONG",
                },
                color,
            ),
            7 => decimal(s, 124, y, v.start_frame as i32, color),
            8 => decimal(s, 124, y, v.end_frame as i32, color),
            9 => decimal(s, 124, y, v.attack_ms as i32, color),
            10 => decimal(s, 124, y, v.decay_ms as i32, color),
            11 => decimal(s, 124, y, v.sustain as i32, color),
            12 => decimal(s, 124, y, v.release_ms as i32, color),
            _ => {}
        }
    }
}

fn placeholder(s: &mut Surface<'_>, screen: Screen, p: &Palette) {
    s.text(28, 72, "UI-002 PLACEHOLDER", p.secondary);
    s.text(28, 92, screen_name(screen), p.text);
    s.text(28, 112, "CORE EDITOR SCREENS READY", p.muted);
}
fn minimap(s: &mut Surface<'_>, screen: Screen, p: &Palette) {
    let x = 258;
    let y = 228;
    for i in 0..5 {
        s.fill_rect(
            Rect {
                x: x + i * 10,
                y,
                width: 6,
                height: 6,
            },
            if i == core_position(screen) {
                p.accent
            } else {
                p.muted
            },
        );
    }
}
const fn core_position(s: Screen) -> i32 {
    match s {
        Screen::Live => 0,
        Screen::Song => 1,
        Screen::Chain => 2,
        Screen::Phrase => 3,
        Screen::Instrument => 4,
        _ => -1,
    }
}
const fn screen_name(s: Screen) -> &'static str {
    match s {
        Screen::Live => "LIVE",
        Screen::Song => "SONG",
        Screen::Chain => "CHAIN",
        Screen::Phrase => "PHRASE",
        Screen::Instrument => "INSTRUMENT",
        Screen::Table => "TABLE",
        Screen::Groove => "GROOVE",
        Screen::Mixer => "MIXER",
        Screen::Effects => "EFFECTS",
        Screen::Project => "PROJECT",
        Screen::Settings => "SETTINGS",
        Screen::SampleBrowser => "SAMPLE BROWSER",
        Screen::SampleEditor => "SAMPLE EDITOR",
        Screen::Diagnostics => "DIAGNOSTICS",
    }
}
fn effect(s: &mut Surface<'_>, x: i32, y: i32, e: crate::model::Effect, color: Rgb565) {
    if let Some(cmd) = e.command {
        let mut b = [cmd, b' ', 0, 0];
        hex(e.value, &mut b[2..]);
        text_bytes(s, x, y, &b, color)
    } else {
        s.text(x, y, "--- --", color)
    }
}
fn note(s: &mut Surface<'_>, x: i32, y: i32, n: Note, color: Rgb565) {
    match n {
        Note::Empty => s.text(x, y, "---", color),
        Note::Off => s.text(x, y, "OFF", color),
        Note::Pitch(v) => {
            let names = [
                b'C', b'C', b'D', b'D', b'E', b'F', b'F', b'G', b'G', b'A', b'A', b'B',
            ];
            let sharp = matches!(v % 12, 1 | 3 | 6 | 8 | 10);
            let b = [
                names[(v % 12) as usize],
                if sharp { b'#' } else { b'-' },
                digit(v / 12),
            ];
            text_bytes(s, x, y, &b, color)
        }
    }
}
fn signed(s: &mut Surface<'_>, x: i32, y: i32, v: i8, color: Rgb565) {
    let magnitude = v.unsigned_abs();
    let mut b = [if v < 0 { b'-' } else { b'+' }, 0, 0];
    hex(magnitude, &mut b[1..]);
    text_bytes(s, x, y, &b, color)
}
fn decimal(s: &mut Surface<'_>, x: i32, y: i32, v: i32, color: Rgb565) {
    let mut b = [b' '; 11];
    let negative = v < 0;
    let mut n = v.unsigned_abs();
    let mut at = 11;
    loop {
        at -= 1;
        b[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if negative {
        at -= 1;
        b[at] = b'-'
    }
    text_bytes(s, x, y, &b[at..], color)
}
fn hex(v: u8, out: &mut [u8]) {
    const H: &[u8; 16] = b"0123456789ABCDEF";
    out[0] = H[(v >> 4) as usize];
    out[1] = H[(v & 15) as usize]
}
fn hex16(v: u16, out: &mut [u8; 4]) {
    hex((v >> 8) as u8, &mut out[..2]);
    hex(v as u8, &mut out[2..])
}
const fn digit(v: u8) -> u8 {
    if v < 10 { b'0' + v } else { b'A' + v - 10 }
}
fn text_bytes(s: &mut Surface<'_>, x: i32, y: i32, b: &[u8], color: Rgb565) {
    if let Ok(text) = core::str::from_utf8(b) {
        s.text(x, y, text, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_all_screens_and_clips() {
        let mut c = Controller::example();
        let p = Palette::default();
        let mut a = [0u16; WIDTH * HEIGHT];
        for screen in [
            Screen::Song,
            Screen::Chain,
            Screen::Phrase,
            Screen::Instrument,
            Screen::SampleBrowser,
        ] {
            c.screen = screen;
            render(&c, &mut a, &p);
            assert!(a.iter().any(|pixel| *pixel == p.accent.0));
        }
        let mut tiny = [0u16; 16];
        let mut surface = Surface::new(&mut tiny, 4, 4);
        surface.fill_rect(
            Rect {
                x: -5,
                y: -5,
                width: 20,
                height: 20,
            },
            p.text,
        );
        assert!(tiny.iter().all(|v| *v == p.text.0));
    }
}

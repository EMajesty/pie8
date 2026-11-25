use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use pie8_ui::{
    Button, ButtonEvent, Controller, EventKind, HEIGHT, Input, Palette, Screen, WIDTH, render,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

struct Options {
    headless: bool,
    scale: usize,
    screen: Option<Screen>,
    replay: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    screenshot_dir: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse()?;
    if options.headless || options.replay.is_some() || options.screenshot_dir.is_some() {
        run_headless(options)
    } else {
        run_window(options)
    }
}

fn run_headless(o: Options) -> Result<(), Box<dyn std::error::Error>> {
    let mut controller = Controller::example();
    if let Some(screen) = o.screen {
        controller.screen = screen
    }
    let mut input = Input::default();
    if let Some(path) = o.replay.as_deref() {
        replay(path, &mut input, &mut controller)?
    }
    let palette = Palette::default();
    let mut pixels = vec![0u16; WIDTH * HEIGHT];
    if let Some(dir) = o.screenshot_dir {
        fs::create_dir_all(&dir)?;
        for (name, screen) in [
            ("song", Screen::Song),
            ("chain", Screen::Chain),
            ("phrase", Screen::Phrase),
            ("instrument", Screen::Instrument),
        ] {
            controller.screen = screen;
            render(&controller, &mut pixels, &palette);
            write_png(&dir.join(format!("{name}.png")), &pixels)?;
        }
    } else {
        render(&controller, &mut pixels, &palette);
        if let Some(path) = o.screenshot {
            write_png(&path, &pixels)?;
        }
    }
    println!(
        "frame {:016x} screen {:?}",
        hash(&pixels),
        controller.screen
    );
    Ok(())
}

fn run_window(o: Options) -> Result<(), Box<dyn std::error::Error>> {
    let mut controller = Controller::example();
    if let Some(screen) = o.screen {
        controller.screen = screen
    }
    let mut input = Input::default();
    let mut rgb565 = vec![0u16; WIDTH * HEIGHT];
    let mut display = vec![0u32; WIDTH * HEIGHT];
    let scale = match o.scale {
        1 => Scale::X1,
        2 => Scale::X2,
        4 => Scale::X4,
        8 => Scale::X8,
        _ => Scale::X4,
    };
    let mut window = Window::new(
        "PIE8 tracker simulator",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            scale,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(120);
    let started = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = started.elapsed().as_micros() as u64;
        for key in window.get_keys_pressed(KeyRepeat::No) {
            if let Some(button) = map_key(key) {
                dispatch(
                    &mut input,
                    &mut controller,
                    ButtonEvent {
                        button,
                        kind: EventKind::Press,
                        timestamp_us: now,
                    },
                )
            }
        }
        for key in window.get_keys_released() {
            if let Some(button) = map_key(key) {
                dispatch(
                    &mut input,
                    &mut controller,
                    ButtonEvent {
                        button,
                        kind: EventKind::Release,
                        timestamp_us: now,
                    },
                )
            }
        }
        while let Some(action) = input.tick(now) {
            controller.action(action)
        }
        render(&controller, &mut rgb565, &Palette::default());
        for (dst, src) in display.iter_mut().zip(&rgb565) {
            *dst = rgb(*src)
        }
        window.update_with_buffer(&display, WIDTH, HEIGHT)?;
    }
    if let Some(path) = o.screenshot {
        write_png(&path, &rgb565)?
    }
    Ok(())
}

fn dispatch(input: &mut Input, controller: &mut Controller, event: ButtonEvent) {
    if let Some(action) = input.event(event) {
        controller.action(action)
    }
}
fn map_key(key: Key) -> Option<Button> {
    match key {
        Key::Up | Key::W => Some(Button::Up),
        Key::Down | Key::S => Some(Button::Down),
        Key::Left | Key::A => Some(Button::Left),
        Key::Right | Key::D => Some(Button::Right),
        Key::LeftShift | Key::J => Some(Button::Shift),
        Key::X | Key::K => Some(Button::Edit),
        Key::Z | Key::L => Some(Button::Option),
        Key::Space | Key::Enter => Some(Button::Play),
        _ => None,
    }
}

fn replay(
    path: &Path,
    input: &mut Input,
    controller: &mut Controller,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let mut last = 0;
    for (line_no, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let time: u64 = fields.next().ok_or("missing time")?.parse()?;
        if time < last {
            return Err(format!("line {} time moved backwards", line_no + 1).into());
        }
        while let Some(action) = input.tick(time) {
            controller.action(action)
        }
        let kind = match fields.next() {
            Some("PRESS") => EventKind::Press,
            Some("RELEASE") => EventKind::Release,
            _ => return Err(format!("line {} invalid event", line_no + 1).into()),
        };
        let button =
            parse_button(fields.next().ok_or("missing button")?).ok_or("invalid button")?;
        if fields.next().is_some() {
            return Err(format!("line {} trailing data", line_no + 1).into());
        }
        dispatch(
            input,
            controller,
            ButtonEvent {
                button,
                kind,
                timestamp_us: time,
            },
        );
        last = time
    }
    Ok(())
}
fn parse_button(value: &str) -> Option<Button> {
    match value {
        "UP" => Some(Button::Up),
        "DOWN" => Some(Button::Down),
        "LEFT" => Some(Button::Left),
        "RIGHT" => Some(Button::Right),
        "SHIFT" => Some(Button::Shift),
        "EDIT" => Some(Button::Edit),
        "OPTION" => Some(Button::Option),
        "PLAY" => Some(Button::Play),
        _ => None,
    }
}

fn write_png(path: &Path, pixels: &[u16]) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let mut bytes = vec![0u8; pixels.len() * 3];
    for (out, pixel) in bytes.chunks_exact_mut(3).zip(pixels) {
        let c = rgb(*pixel);
        out.copy_from_slice(&[
            ((c >> 16) & 255) as u8,
            ((c >> 8) & 255) as u8,
            (c & 255) as u8,
        ]);
    }
    writer.write_image_data(&bytes)?;
    Ok(())
}
fn rgb(v: u16) -> u32 {
    let r = ((v >> 11) & 31) as u32;
    let g = ((v >> 5) & 63) as u32;
    let b = (v & 31) as u32;
    ((r * 255 / 31) << 16) | ((g * 255 / 63) << 8) | (b * 255 / 31)
}
fn hash(pixels: &[u16]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for v in pixels {
        for byte in v.to_le_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3)
        }
    }
    h
}

fn parse() -> Result<Options, Box<dyn std::error::Error>> {
    let mut o = Options {
        headless: false,
        scale: 4,
        screen: None,
        replay: None,
        screenshot: None,
        screenshot_dir: None,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => o.headless = true,
            "--scale" => {
                o.scale = args.next().ok_or("missing scale")?.parse()?;
                if !matches!(o.scale, 1 | 2 | 4 | 8) {
                    return Err("scale must be 1, 2, 4, or 8".into());
                }
            }
            "--screen" => {
                o.screen = Some(
                    parse_screen(&args.next().ok_or("missing screen")?).ok_or("invalid screen")?,
                )
            }
            "--replay" => o.replay = Some(args.next().ok_or("missing replay")?.into()),
            "--screenshot" => o.screenshot = Some(args.next().ok_or("missing screenshot")?.into()),
            "--screenshot-dir" => {
                o.screenshot_dir = Some(args.next().ok_or("missing screenshot dir")?.into())
            }
            "--help" => {
                println!(
                    "pie8-sim [--headless] [--scale 1|2|4|8] [--screen song|chain|phrase|instrument] [--replay FILE] [--screenshot FILE] [--screenshot-dir DIR]"
                );
                std::process::exit(0)
            }
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    Ok(o)
}
fn parse_screen(v: &str) -> Option<Screen> {
    match v {
        "song" => Some(Screen::Song),
        "chain" => Some(Screen::Chain),
        "phrase" => Some(Screen::Phrase),
        "instrument" => Some(Screen::Instrument),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_replay_hash() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("replays/edit-session.txt");
        let mut a = Controller::example();
        let mut ia = Input::default();
        replay(&path, &mut ia, &mut a).unwrap();
        let mut pa = vec![0u16; WIDTH * HEIGHT];
        render(&a, &mut pa, &Palette::default());
        let mut b = Controller::example();
        let mut ib = Input::default();
        replay(&path, &mut ib, &mut b).unwrap();
        let mut pb = vec![0u16; WIDTH * HEIGHT];
        render(&b, &mut pb, &Palette::default());
        assert_eq!(a.project, b.project);
        assert_eq!(pa, pb);
        assert_eq!(hash(&pa), hash(&pb));
        assert_eq!(hash(&pa), 0x9ab6_47b1_9df2_00e0);
    }
}

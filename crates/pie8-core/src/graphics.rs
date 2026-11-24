#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Rgb565(pub u16);

impl Rgb565 {
    pub const BLACK: Self = Self(0x0000);
    pub const WHITE: Self = Self(0xffff);
    pub const RED: Self = Self(0xf800);
    pub const GREEN: Self = Self(0x07e0);
    pub const BLUE: Self = Self(0x001f);
    pub const YELLOW: Self = Self(0xffe0);
    pub const CYAN: Self = Self(0x07ff);
    pub const MAGENTA: Self = Self(0xf81f);

    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self(((red as u16 & 0xf8) << 8) | ((green as u16 & 0xfc) << 3) | (blue as u16 >> 3))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct Surface<'a> {
    pixels: &'a mut [u16],
    width: u32,
    height: u32,
}

impl<'a> Surface<'a> {
    pub fn new(pixels: &'a mut [u16], width: u32, height: u32) -> Self {
        assert!(pixels.len() >= width as usize * height as usize);
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn clear(&mut self, color: Rgb565) {
        self.pixels.fill(color.0);
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Rgb565) {
        let x0 = rect.x.max(0).min(self.width as i32) as usize;
        let y0 = rect.y.max(0).min(self.height as i32) as usize;
        let x1 = rect
            .x
            .saturating_add_unsigned(rect.width)
            .max(0)
            .min(self.width as i32) as usize;
        let y1 = rect
            .y
            .saturating_add_unsigned(rect.height)
            .max(0)
            .min(self.height as i32) as usize;
        for row in y0..y1 {
            self.pixels[row * self.width as usize + x0..row * self.width as usize + x1]
                .fill(color.0);
        }
    }

    pub fn horizontal_line(&mut self, x: i32, y: i32, length: u32, color: Rgb565) {
        self.fill_rect(
            Rect {
                x,
                y,
                width: length,
                height: 1,
            },
            color,
        );
    }

    pub fn vertical_line(&mut self, x: i32, y: i32, length: u32, color: Rgb565) {
        self.fill_rect(
            Rect {
                x,
                y,
                width: 1,
                height: length,
            },
            color,
        );
    }

    pub fn text(&mut self, mut x: i32, y: i32, text: &str, color: Rgb565) {
        for character in text.bytes() {
            self.glyph(x, y, character.to_ascii_uppercase(), color);
            x += 6;
        }
    }

    fn glyph(&mut self, x: i32, y: i32, character: u8, color: Rgb565) {
        let rows = glyph(character);
        for (dy, row) in rows.into_iter().enumerate() {
            for dx in 0..5 {
                if row & (1 << (4 - dx)) != 0 {
                    self.fill_rect(
                        Rect {
                            x: x + dx,
                            y: y + dy as i32,
                            width: 1,
                            height: 1,
                        },
                        color,
                    );
                }
            }
        }
    }
}

#[rustfmt::skip]
fn glyph(c: u8) -> [u8; 7] {
    match c {
        b'A' => [14,17,17,31,17,17,17], b'B' => [30,17,17,30,17,17,30],
        b'C' => [14,17,16,16,16,17,14], b'D' => [30,17,17,17,17,17,30],
        b'E' => [31,16,16,30,16,16,31], b'F' => [31,16,16,30,16,16,16],
        b'G' => [14,17,16,23,17,17,15], b'H' => [17,17,17,31,17,17,17],
        b'I' => [14,4,4,4,4,4,14], b'J' => [1,1,1,1,17,17,14],
        b'K' => [17,18,20,24,20,18,17], b'L' => [16,16,16,16,16,16,31],
        b'M' => [17,27,21,21,17,17,17], b'N' => [17,25,21,19,17,17,17],
        b'O' => [14,17,17,17,17,17,14], b'P' => [30,17,17,30,16,16,16],
        b'Q' => [14,17,17,17,21,18,13], b'R' => [30,17,17,30,20,18,17],
        b'S' => [15,16,16,14,1,1,30], b'T' => [31,4,4,4,4,4,4],
        b'U' => [17,17,17,17,17,17,14], b'V' => [17,17,17,17,17,10,4],
        b'W' => [17,17,17,21,21,21,10], b'X' => [17,17,10,4,10,17,17],
        b'Y' => [17,17,10,4,4,4,4], b'Z' => [31,1,2,4,8,16,31],
        b'0' => [14,17,19,21,25,17,14], b'1' => [4,12,4,4,4,4,14],
        b'2' => [14,17,1,2,4,8,31], b'3' => [30,1,1,14,1,1,30],
        b'4' => [2,6,10,18,31,2,2], b'5' => [31,16,16,30,1,1,30],
        b'6' => [14,16,16,30,17,17,14], b'7' => [31,1,2,4,8,8,8],
        b'8' => [14,17,17,14,17,17,14], b'9' => [14,17,17,15,1,1,14],
        b':' => [0,4,4,0,4,4,0], b'.' => [0,0,0,0,0,4,4],
        b'-' => [0,0,0,31,0,0,0], b'/' => [1,1,2,4,8,16,16],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_rectangles_on_every_edge() {
        let mut pixels = [0; 16];
        let mut surface = Surface::new(&mut pixels, 4, 4);
        surface.fill_rect(
            Rect {
                x: -2,
                y: 1,
                width: 4,
                height: 8,
            },
            Rgb565::WHITE,
        );
        assert_eq!(
            pixels,
            [
                0, 0, 0, 0, 0xffff, 0xffff, 0, 0, 0xffff, 0xffff, 0, 0, 0xffff, 0xffff, 0, 0
            ]
        );
    }
}

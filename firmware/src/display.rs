use pie8_core::graphics::Rect;
use pie8_rpi::{
    gpio::{Function, Pin},
    spi::{Spi0, SpiError},
    timer::Timer,
};

#[derive(Clone, Copy)]
#[allow(dead_code)] // Board configuration intentionally exposes every supported orientation.
pub enum Orientation {
    Portrait,
    Landscape,
    PortraitInverted,
    LandscapeInverted,
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // Prototype defaults to RGB; BGR is a documented panel diagnostic switch.
pub enum ColorOrder {
    Rgb,
    Bgr,
}

#[derive(Clone, Copy)]
pub struct PanelConfig {
    pub width: u16,
    pub height: u16,
    pub x_offset: u16,
    pub y_offset: u16,
    pub orientation: Orientation,
    pub inverted: bool,
    pub color_order: ColorOrder,
}

pub struct St7789 {
    spi: Spi0,
    dc: Pin,
    reset: Pin,
    backlight: Pin,
    config: PanelConfig,
}

impl St7789 {
    pub fn new(spi: Spi0, dc: Pin, reset: Pin, backlight: Pin, config: PanelConfig) -> Self {
        for pin in [dc, reset, backlight] {
            pin.set_low();
            pin.set_function(Function::Output);
        }
        Self {
            spi,
            dc,
            reset,
            backlight,
            config,
        }
    }

    pub fn init(&mut self, timer: &Timer) -> Result<(), SpiError> {
        self.backlight.set_low();
        self.reset.set_high();
        timer.delay_ms(10);
        self.reset.set_low();
        timer.delay_ms(20);
        self.reset.set_high();
        timer.delay_ms(120);

        self.command(timer, 0x01, &[])?; // SWRESET
        timer.delay_ms(150);
        self.command(timer, 0x11, &[])?; // SLPOUT
        timer.delay_ms(120);
        self.command(timer, 0x3a, &[0x55])?; // COLMOD: RGB565
        let mut madctl = match self.config.orientation {
            Orientation::Portrait => 0x00,
            Orientation::Landscape => 0x60,         // MX | MV
            Orientation::PortraitInverted => 0xc0,  // MX | MY
            Orientation::LandscapeInverted => 0xa0, // MY | MV
        };
        if matches!(self.config.color_order, ColorOrder::Bgr) {
            madctl |= 0x08;
        }
        self.command(timer, 0x36, &[madctl])?; // MADCTL
        self.command(timer, if self.config.inverted { 0x21 } else { 0x20 }, &[])?;
        self.command(timer, 0x13, &[])?; // NORON
        self.command(timer, 0x29, &[])?; // DISPON
        timer.delay_ms(20);
        self.backlight.set_high();
        Ok(())
    }

    pub fn set_inverted(&mut self, timer: &Timer, inverted: bool) -> Result<(), SpiError> {
        self.config.inverted = inverted;
        self.command(timer, if inverted { 0x21 } else { 0x20 }, &[])
    }

    pub fn set_color_order(&mut self, timer: &Timer, order: ColorOrder) -> Result<(), SpiError> {
        self.config.color_order = order;
        let mut madctl = match self.config.orientation {
            Orientation::Portrait => 0x00,
            Orientation::Landscape => 0x60,
            Orientation::PortraitInverted => 0xc0,
            Orientation::LandscapeInverted => 0xa0,
        };
        if matches!(order, ColorOrder::Bgr) {
            madctl |= 0x08;
        }
        self.command(timer, 0x36, &[madctl])
    }

    pub fn set_spi_divider(&mut self, divider: u16) {
        self.spi.set_divider(divider);
    }

    pub fn update_region(
        &mut self,
        timer: &Timer,
        framebuffer: &[u16],
        stride: usize,
        rect: Rect,
    ) -> Result<(), SpiError> {
        let x = rect.x.max(0) as u16;
        let y = rect.y.max(0) as u16;
        let width = rect
            .width
            .min(u32::from(self.config.width.saturating_sub(x))) as u16;
        let height = rect
            .height
            .min(u32::from(self.config.height.saturating_sub(y))) as u16;
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.set_window(timer, x, y, width, height)?;
        self.dc.set_high();
        for row in y as usize..usize::from(y + height) {
            let start = row * stride + x as usize;
            self.spi
                .write_pixels(timer, &framebuffer[start..start + width as usize])?;
        }
        Ok(())
    }

    fn set_window(
        &mut self,
        timer: &Timer,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> Result<(), SpiError> {
        let x0 = x + self.config.x_offset;
        let y0 = y + self.config.y_offset;
        let x1 = x0 + width - 1;
        let y1 = y0 + height - 1;
        self.command(
            timer,
            0x2a,
            &[x0.to_be_bytes(), x1.to_be_bytes()].concat_array(),
        )?;
        self.command(
            timer,
            0x2b,
            &[y0.to_be_bytes(), y1.to_be_bytes()].concat_array(),
        )?;
        self.command(timer, 0x2c, &[])
    }

    fn command(&mut self, timer: &Timer, command: u8, data: &[u8]) -> Result<(), SpiError> {
        self.dc.set_low();
        self.spi.write(timer, &[command])?;
        if !data.is_empty() {
            self.dc.set_high();
            self.spi.write(timer, data)?;
        }
        Ok(())
    }
}

trait ConcatArray {
    fn concat_array(self) -> [u8; 4];
}

impl ConcatArray for [[u8; 2]; 2] {
    fn concat_array(self) -> [u8; 4] {
        [self[0][0], self[0][1], self[1][0], self[1][1]]
    }
}

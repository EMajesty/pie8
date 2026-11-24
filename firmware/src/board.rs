use pie8_rpi::gpio::Pin;

/// Prototype wiring. GPIO values use BCM numbering; comments give header pins.
pub const LCD_SCK: Pin = Pin::new(11); // physical 23
pub const LCD_MOSI: Pin = Pin::new(10); // physical 19
pub const LCD_CS: Pin = Pin::new(8); // physical 24, SPI0 CE0
pub const LCD_DC: Pin = Pin::new(25); // physical 22
pub const LCD_RESET: Pin = Pin::new(24); // physical 18
pub const LCD_BACKLIGHT: Pin = Pin::new(18); // physical 12
pub const BUTTONS: [Pin; 8] = [
    Pin::new(21), // button 1, physical 40
    Pin::new(20), // button 2, physical 38
    Pin::new(26), // button 3, physical 37
    Pin::new(16), // button 4, physical 36
    Pin::new(19), // button 5, physical 35
    Pin::new(13), // button 6, physical 33
    Pin::new(12), // button 7, physical 32
    Pin::new(6),  // button 8, physical 31
];
pub const UART_TX: Pin = Pin::new(14); // physical 8
pub const UART_RX: Pin = Pin::new(15); // physical 10

pub const CORE_CLOCK_HZ: u32 = 250_000_000;
/// Even divider 32 gives 7.8125 MHz from the fixed 250 MHz core clock.
pub const SPI_CLOCK_DIVIDER: u16 = 32;
pub const SPI_TIMEOUT_US: u64 = 100_000;

// Keep these explicit: clone modules often require different values.
pub const DISPLAY_CONFIG: crate::display::PanelConfig = crate::display::PanelConfig {
    width: 320,
    height: 240,
    x_offset: 0,
    y_offset: 0,
    orientation: crate::display::Orientation::Landscape,
    inverted: true,
    color_order: crate::display::ColorOrder::Rgb,
};

// These constants make the complete pin map visible to dead-code/static review.
pub const fn assert_wiring() {
    let _ = (LCD_SCK, LCD_MOSI, LCD_CS, UART_TX, UART_RX);
}

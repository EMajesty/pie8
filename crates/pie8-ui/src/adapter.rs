use crate::{ButtonEvent, HEIGHT, WIDTH};

/// Minimal firmware/desktop boundary. Implementations own clocks, buttons and
/// framebuffer presentation; tracker state and rendering remain portable.
pub trait Platform {
    fn now_us(&self) -> u64;
    fn next_button_event(&mut self) -> Option<ButtonEvent>;
    fn framebuffer(&mut self) -> &mut [u16; WIDTH * HEIGHT];
    fn present(&mut self);
}

use embedded_graphics::pixelcolor::Gray4;

use crate::BmpImage;

pub const DISPLAY_WIDTH: i32 = 960;
pub const DISPLAY_HEIGHT: i32 = 540;

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("display error: {0}")]
    Embedded(i32),
}

pub trait DisplayTarget {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn clear(&mut self, color: Gray4);
    fn flush(&mut self) -> Result<(), DisplayError>;
    fn draw_pixel(&mut self, x: i32, y: i32, nibble: u8);
    fn draw_bitmap(&mut self, image: &BmpImage, x: i32, y: i32);
    fn poll_events(&mut self) -> bool {
        false
    }
}

#[cfg(target_os = "espidf")]
#[path = "display_embedded.rs"]
mod display_embedded;
#[cfg(target_os = "espidf")]
pub use display_embedded::{
    EmbeddedDisplay, ScaledDisplay, display_begin, display_commit, display_draw_bitmap,
    display_draw_rect, display_draw_text, set_display_font_size, set_display_rotation,
};

#[cfg(feature = "simulator")]
#[path = "display_sim.rs"]
mod display_sim;
#[cfg(feature = "simulator")]
pub use display_sim::SimulatedDisplay;

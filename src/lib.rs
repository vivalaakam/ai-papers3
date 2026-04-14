extern crate alloc;

mod clock;
mod config;
mod display;
mod enums;
mod file;
mod image;
mod wifi;

pub use clock::Clock;
pub use config::Config;
pub use display::{
    display_begin, display_commit, display_draw_bitmap, display_draw_rect, display_draw_text,
    set_display_font_size, set_display_rotation,
};
pub use enums::MainAppError;
pub use file::read_file;
pub use image::{BmpImage, load_image};
pub use wifi::{WifiConnection, connect_wifi_networks};

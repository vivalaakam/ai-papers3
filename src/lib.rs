extern crate alloc;

mod clock;
pub mod ui;
mod config;
mod display;
mod enums;
mod file;
mod image;
mod touch;
mod wifi;

pub use clock::Clock;
pub use config::Config;
pub use display::{
    display_begin, display_commit, display_draw_bitmap, display_draw_rect, display_draw_text,
    set_display_font_size, set_display_rotation, EmbeddedDisplay, ScaledDisplay,
};
pub use enums::MainAppError;
pub use file::read_file;
pub use image::{BmpImage, load_image};
pub use touch::{Gt911, TouchEvent, TouchPoint, TouchTracker};
pub use wifi::{WifiConnection, connect_wifi_networks};

// UI system
pub use ui::UiApp;
pub use ui::canvas::{Color, FontSize, UiRect};
pub use ui::events::UiEvent;
pub use ui::components::view::{
    AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue, View, ViewProps,
};
pub use ui::components::text::{Text, TextAlign, TextProps};
pub use ui::components::button::{Button, ButtonProps};
pub use ui::components::image::{Image, ImageProps};

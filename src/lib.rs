extern crate alloc;

mod clock;
mod config;
mod display;
mod enums;
mod file;
pub mod fonts;
mod image;
mod touch;
pub mod ui;
mod wifi;

pub use clock::Clock;
pub use config::Config;
pub use display::{
    EmbeddedDisplay, ScaledDisplay, display_begin, display_commit, display_draw_bitmap,
    display_draw_rect, display_draw_text, set_display_font_size, set_display_rotation,
};
pub use enums::MainAppError;
pub use file::read_file;
pub use image::{BmpImage, load_image};
pub use touch::{Gt911, TouchEvent, TouchPoint, TouchTracker};
pub use wifi::{WifiConnection, connect_wifi_networks};

// UI system
pub use ui::UiApp;
pub use ui::canvas::{Color, FontSize, UiRect};
pub use ui::components::button::{Button, ButtonProps};
pub use ui::components::image::{Image, ImageProps};
pub use ui::components::text::{Text, TextAlign, TextProps};
pub use ui::components::view::{
    AlignItems, Border, Direction, EdgeInsets, JustifyContent, SizeValue, View, ViewProps,
};
pub use ui::events::UiEvent;

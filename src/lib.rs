extern crate alloc;

mod app;
#[cfg(target_os = "espidf")]
mod clock;
mod config;
mod display;
mod enums;
pub mod fonts;
mod image;
mod storage;
#[cfg(target_os = "espidf")]
mod touch;
#[cfg(not(target_os = "espidf"))]
#[path = "touch_stub.rs"]
mod touch;
pub mod ui;
#[cfg(target_os = "espidf")]
mod wifi;

pub use app::{
    AppAssets, CONFIG_FILE_NAME, load_assets, render_image_screen, render_text_screen,
    show_loading_stage,
};
#[cfg(target_os = "espidf")]
pub use clock::Clock;
pub use config::Config;
#[cfg(feature = "simulator")]
pub use display::SimulatedDisplay;
pub use display::{DISPLAY_HEIGHT, DISPLAY_WIDTH, DisplayError, DisplayTarget};
#[cfg(target_os = "espidf")]
pub use display::{
    EmbeddedDisplay, ScaledDisplay, display_begin, display_commit, display_draw_bitmap,
    display_draw_rect, display_draw_text, set_display_font_size, set_display_rotation,
};
pub use enums::MainAppError;
pub use image::{BmpImage, load_image};
#[cfg(feature = "simulator")]
pub use storage::LocalStorage;
pub use storage::Storage;
#[cfg(target_os = "espidf")]
pub use touch::{Gt911, TouchEvent, TouchPoint, TouchTracker};
#[cfg(not(target_os = "espidf"))]
pub use touch::{TouchEvent, TouchPoint, TouchTracker};
#[cfg(target_os = "espidf")]
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

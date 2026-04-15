extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;

use crate::element;
use crate::storage::Storage;
use crate::ui::UiApp;
use crate::{
    AlignItems, BmpImage, Color, Config, Direction, EdgeInsets, FontSize, Image, JustifyContent,
    SizeValue, Text, View, load_image,
};
use log::info;

pub const CONFIG_FILE_NAME: &str = "PAPERS~4.YAM";

pub struct AppAssets {
    pub config: Config,
    pub sleep_image: Option<Arc<BmpImage>>,
}

pub fn load_assets(storage: &dyn Storage) -> Result<AppAssets, String> {
    let config_data = storage
        .read_file(CONFIG_FILE_NAME)
        .ok_or_else(|| format!("{} not found", CONFIG_FILE_NAME))?;
    let config = serde_yaml::from_slice::<Config>(&config_data)
        .map_err(|err| format!("YAML parse error: {}", err))?;
    let sleep_image = load_image(storage).map(Arc::new);
    Ok(AppAssets {
        config,
        sleep_image,
    })
}

// ─── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    ui: UiApp,
    storage: Option<Box<dyn Storage>>,
    assets: AppAssets,
}

impl App {
    pub fn new(
        display: impl crate::display::DisplayTarget + 'static,
        storage: impl Storage + 'static,
    ) -> Result<Self, String> {
        let assets = load_assets(&storage)?;
        Ok(Self {
            ui: UiApp::new(display),
            storage: Some(Box::new(storage)),
            assets,
        })
    }

    pub fn with_ui(ui: UiApp, storage: impl Storage + 'static) -> Result<Self, String> {
        let assets = load_assets(&storage)?;
        Ok(Self {
            ui,
            storage: Some(Box::new(storage)),
            assets,
        })
    }

    pub fn into_display(self) -> Box<dyn crate::display::DisplayTarget> {
        self.ui.into_display()
    }

    pub fn storage(&self) -> Option<&dyn Storage> {
        self.storage.as_deref()
    }

    pub fn assets(&self) -> &AppAssets {
        &self.assets
    }

    pub fn render(&mut self, element: crate::ui::element::AnyElement) {
        self.ui.render(element);
    }

    pub fn handle_event(&mut self, event: crate::ui::events::UiEvent) -> bool {
        self.ui.handle_event(event)
    }

    pub fn poll_events(&mut self) -> bool {
        self.ui.poll_events()
    }

    pub fn drain_input(&mut self) -> alloc::vec::Vec<crate::TouchPoint> {
        self.ui.drain_input()
    }

    pub fn ui_mut(&mut self) -> &mut UiApp {
        &mut self.ui
    }
}

// ─── Экраны ─────────────────────────────────────────────────────────────────

/// Экран с текстовыми строками (загрузка, статус, отладка).
/// `lines` — текст с переносами '\n' или без.
pub fn render_text_screen(app: &mut UiApp, lines: &str) {
    app.render(element!(View {
        direction: Direction::Column,
        width: SizeValue::Percent(100.0),
        height: SizeValue::Percent(100.0),
        padding: EdgeInsets::all(16),
        background: Some(Color::WHITE),
        children: vec![element!(Text {
            content: lines.to_string(),
            font_size: FontSize::Large,
            color: Color::BLACK,
        })],
    }));
}

/// Экран с изображением по центру (заставка / sleep screen).
pub fn render_image_screen(app: &mut UiApp, image: Arc<BmpImage>) {
    app.render(element!(View {
        direction: Direction::Column,
        width: SizeValue::Percent(100.0),
        height: SizeValue::Percent(100.0),
        background: Some(Color::WHITE),
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        children: vec![element!(Image { image: Some(image) })],
    }));
}

/// Добавить строку к накапливаемому тексту загрузки и отобразить.
pub fn show_loading_stage(app: &mut UiApp, stage: &str, accumulated: &mut String) {
    info!("{}", stage);
    if !accumulated.is_empty() {
        accumulated.push('\n');
    }
    accumulated.push_str(stage);
    render_text_screen(app, accumulated.as_str());
}

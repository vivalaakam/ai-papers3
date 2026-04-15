use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use ai_papers3::{
    DISPLAY_HEIGHT, DISPLAY_WIDTH, LocalStorage, SimulatedDisplay, UiApp, UiEvent, load_assets,
    render_image_screen, render_text_screen, show_loading_stage,
};

fn main() {
    let mut loading_text = String::new();
    let sdcard_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdcard");
    let storage = LocalStorage::new(sdcard_root);

    let display = SimulatedDisplay::new(DISPLAY_HEIGHT, DISPLAY_WIDTH, 1, 0);
    let mut app = UiApp::new(display);

    show_loading_stage(
        &mut app,
        "Локальный режим\nЗагрузка конфигурации",
        &mut loading_text,
    );

    let assets = match load_assets(&storage) {
        Ok(assets) => assets,
        Err(err) => {
            render_text_screen(&mut app, &format!("Ошибка\n{}", err));
            loop {
                if app.poll_events() {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            return;
        }
    };

    let status_text = format!(
        "Локальный режим\nBackend: {}\nWiFi сетей: {}",
        assets.config.backend_url,
        assets.config.networks.len()
    );
    render_text_screen(&mut app, &status_text);

    if let Some(image) = assets.sleep_image {
        thread::sleep(Duration::from_millis(800));
        render_image_screen(&mut app, image);
    }

    loop {
        if app.poll_events() {
            break;
        }
        for point in app.drain_input() {
            if !app.handle_event(UiEvent::Tap(point)) {
                let text = format!("Touch\nX: {}\nY: {}", point.x, point.y);
                render_text_screen(&mut app, &text);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

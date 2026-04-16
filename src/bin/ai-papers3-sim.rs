use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use ai_papers3::{
    App, DISPLAY_HEIGHT, DISPLAY_WIDTH, LocalStorage, SimulatedDisplay, UiEvent,
    render_image_screen, render_text_screen,
};

fn main() {
    let sdcard_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdcard");
    let storage = LocalStorage::new(sdcard_root);
    let display = SimulatedDisplay::new(DISPLAY_HEIGHT, DISPLAY_WIDTH, 1, 0);

    let mut app = App::new(display, storage).expect("Не удалось загрузить ассеты");

    let status_text = format!(
        "Локальный режим\nBackend: {}\nWiFi сетей: {}",
        app.assets().config.backend_url,
        app.assets().config.networks.len()
    );
    render_text_screen(app.ui_mut(), &status_text);

    if let Some(image) = &app.assets().sleep_image {
        let image = image.clone();
        thread::sleep(Duration::from_millis(800));
        render_image_screen(app.ui_mut(), image);
    }

    loop {
        if app.poll_events() {
            break;
        }
        for event in app.poll_touch() {
            if !app.handle_event(event.clone()) {
                let text = match event {
                    UiEvent::Tap(p) => format!("Touch\nX: {}\nY: {}", p.x, p.y),
                    UiEvent::Slide { from, to } => format!(
                        "Slide\n{}:{} -> {}:{}",
                        from.x, from.y, to.x, to.y
                    ),
                };
                render_text_screen(app.ui_mut(), &text);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}
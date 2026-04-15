#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;

use ai_papers3::{
    AlignItems, BmpImage, Clock, Color, Config, Direction, EdgeInsets,
    EmbeddedDisplay, FontSize, Gt911, Image, JustifyContent, MainAppError, SizeValue,
    Text, TouchEvent, TouchTracker, UiApp, UiEvent, View,
    connect_wifi_networks, load_image, read_file, set_display_rotation,
};
use ai_papers3::element;
use embedded_sdmmc::{SdCard, VolumeManager};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config};
use esp_idf_hal::units::{KiloHertz, MegaHertz};
use log::info;

const SLEEP_IDLE_MS: u32 = 30_000;

// ─── UI-функции ──────────────────────────────────────────────────────────────

/// Экран с текстовыми строками (загрузка, статус, отладка).
/// `lines` — текст с переносами '\n' или без.
fn render_text_screen(app: &mut UiApp, lines: &str) {
    app.render(element!(View {
        direction: Direction::Column,
        width: SizeValue::Percent(100.0),
        height: SizeValue::Percent(100.0),
        padding: EdgeInsets::all(16),
        background: Some(Color::WHITE),
        children: vec![
            element!(Text {
                content: lines.to_string(),
                font_size: FontSize::Large,
                color: Color::BLACK,
            })
        ],
    }));
}

/// Экран с изображением по центру (заставка / sleep screen).
fn render_image_screen(app: &mut UiApp, image: Arc<BmpImage>) {
    app.render(element!(View {
        direction: Direction::Column,
        width: SizeValue::Percent(100.0),
        height: SizeValue::Percent(100.0),
        background: Some(Color::WHITE),
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        children: vec![
            element!(Image {
                image: Some(image),
            })
        ],
    }));
}

/// Добавить строку к накапливаемому тексту загрузки и отобразить.
fn show_loading_stage(app: &mut UiApp, stage: &str, accumulated: &mut String) {
    info!("{}", stage);
    if !accumulated.is_empty() {
        accumulated.push('\n');
    }
    accumulated.push_str(stage);
    render_text_screen(app, accumulated.as_str());
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), MainAppError> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let mut loading_text = String::new();

    if let Err(err) = set_display_rotation(270) {
        info!("Display rotation error: {}", err);
    }

    let display = match EmbeddedDisplay::new() {
        Ok(d) => d,
        Err(err) => {
            info!("Display init error: {}", err);
            return Ok(());
        }
    };

    // UiApp берёт ownership над display — всё рисование теперь через него
    let mut app = UiApp::new(display);

    show_loading_stage(&mut app, "Загрузка 1/4\nИнициализация SPI", &mut loading_text);

    let peripherals = peripherals::Peripherals::take().unwrap();
    let modem = peripherals.modem;
    let gpios = peripherals.pins;

    let spi = peripherals.spi2;
    let driver = match SpiDriver::new::<SPI2>(
        spi,
        gpios.gpio39,
        gpios.gpio38,
        Some(gpios.gpio40),
        &SpiDriverConfig::new(),
    ) {
        Ok(d) => d,
        Err(e) => {
            info!("SPI Driver error: {}", e);
            return Ok(());
        }
    };

    let spi_device_config = config::Config::new().baudrate(MegaHertz(10).into());
    let spi_device = match SpiDeviceDriver::new(driver, Some(gpios.gpio47), &spi_device_config) {
        Ok(d) => d,
        Err(e) => {
            info!("SPI Device error: {}", e);
            return Ok(());
        }
    };

    let sdcard = SdCard::new(spi_device, esp_idf_hal::delay::FreeRtos);
    info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    show_loading_stage(&mut app, "Загрузка 2/4\nИнициализация SD", &mut loading_text);

    let volume_mgr = VolumeManager::new(sdcard, Clock);
    let volume0 = match volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)) {
        Ok(d) => d,
        Err(e) => {
            info!("Volume 0 error: {:?}", e);
            return Ok(());
        }
    };

    show_loading_stage(&mut app, "Загрузка 3/4\nЧтение файлов", &mut loading_text);

    let root_dir = match volume0.open_root_dir() {
        Ok(d) => d,
        Err(e) => {
            info!("Root dir error: {:?}", e);
            return Ok(());
        }
    };

    root_dir
        .iterate_dir(|entry| {
            info!(
                "{:12} {:9} {} {}",
                entry.name,
                entry.size,
                entry.mtime,
                if entry.attributes.is_directory() { "<DIR>" } else { "" }
            );
        })
        .unwrap();

    let config = match read_file(&root_dir, "PAPERS~4.YAM") {
        Some(data) => match serde_yaml::from_slice::<Config>(&data) {
            Ok(c) => c,
            Err(err) => {
                info!("YAML parse error: {}", err);
                return Ok(());
            }
        },
        None => {
            info!("PAPERS~4.YAM not found");
            return Ok(());
        }
    };

    info!("Config: {:?}", config);

    // Загружаем sleep-image и оборачиваем в Arc для Image-компонента
    let sleep_image: Option<Arc<BmpImage>> = load_image(&root_dir).map(Arc::new);

    show_loading_stage(&mut app, "Загрузка 4/4\nПодключение WiFi", &mut loading_text);

    let wifi_connection = connect_wifi_networks(modem, &config.networks);

    let status_text = match wifi_connection.as_ref() {
        Some(conn) => alloc::format!("Готово\nWiFi: {}\nIP: {}", conn.ssid, conn.ip),
        None => "Готово\nWiFi: нет сети\nIP: --".to_string(),
    };
    render_text_screen(&mut app, &status_text);

    info!("Initialization complete");

    let i2c_config = I2cConfig::new().baudrate(KiloHertz(400).into());
    let mut i2c = match I2cDriver::new(peripherals.i2c0, gpios.gpio41, gpios.gpio42, &i2c_config) {
        Ok(d) => d,
        Err(err) => {
            info!("I2C init error: {}", err);
            return Ok(());
        }
    };

    let touch_address = match Gt911::detect_address(&mut i2c) {
        Some(addr) => addr,
        None => {
            info!("GT911 not found");
            return Ok(());
        }
    };

    let mut touch = Gt911::new(i2c, touch_address);
    let mut touch_tracker = TouchTracker::new(6);
    let mut idle_ms: u32 = 0;
    let mut sleep_shown = false;

    let touch_int = match PinDriver::input(gpios.gpio48) {
        Ok(d) => d,
        Err(err) => {
            info!("Touch INT pin error: {}", err);
            return Ok(());
        }
    };

    loop {
        if touch_int.is_low() {
            match touch.read_touch() {
                Ok(point) => {
                    if let Some(event) = touch_tracker.on_sample(point) {
                        idle_ms = 0;
                        sleep_shown = false;

                        // Передаём событие в UI (на будущее — для Button)
                        let ui_event = match event {
                            TouchEvent::Touch(p) => Some(UiEvent::Tap(p)),
                            TouchEvent::Slide { from, to } => {
                                Some(UiEvent::Slide { from, to })
                            }
                        };

                        if let Some(ref ev) = ui_event {
                            // Если никто не обработал — показываем координаты
                            if !app.handle_event(ev.clone()) {
                                let text = match event {
                                    TouchEvent::Touch(p) => {
                                        alloc::format!("Touch\nX: {}\nY: {}", p.x, p.y)
                                    }
                                    TouchEvent::Slide { from, to } => alloc::format!(
                                        "Slide\n{}:{} -> {}:{}",
                                        from.x, from.y, to.x, to.y
                                    ),
                                };
                                render_text_screen(&mut app, &text);
                            }
                        }
                    }
                }
                Err(err) => {
                    info!("Touch read error: {}", err);
                }
            }
        } else {
            touch_tracker.on_sample(None);
            idle_ms = idle_ms.saturating_add(50);
            if !sleep_shown && idle_ms >= SLEEP_IDLE_MS {
                sleep_shown = true;
                match sleep_image.as_ref() {
                    Some(img) => render_image_screen(&mut app, Arc::clone(img)),
                    None => {
                        info!("Sleep image not found");
                        render_text_screen(&mut app, "Экран выкл.");
                    }
                }
            }
        }

        FreeRtos::delay_ms(50);
    }
}

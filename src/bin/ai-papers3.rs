#![no_std]
extern crate alloc;

use alloc::string::{String, ToString};

use ai_papers3::{
    App, Clock, EmbeddedDisplay, Gt911, MainAppError, SDCardStorage, UiApp,
    UiEvent, connect_wifi_networks, render_image_screen, render_text_screen,
    set_display_rotation, show_loading_stage,
};
use embedded_sdmmc::{SdCard, VolumeManager};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config};
use esp_idf_hal::units::{KiloHertz, MegaHertz};
use log::info;

const SLEEP_IDLE_MS: u32 = 30_000;

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

    let mut ui = UiApp::new(display);

    show_loading_stage(
        &mut ui,
        "Загрузка 1/4\nИнициализация SPI",
        &mut loading_text,
    );

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

    let sdcard = SdCard::new(spi_device, FreeRtos);
    info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    show_loading_stage(
        &mut ui,
        "Загрузка 2/4\nИнициализация SD",
        &mut loading_text,
    );

    let volume_mgr = VolumeManager::new(sdcard, Clock);
    let storage = SDCardStorage::new(volume_mgr, None);

    show_loading_stage(&mut ui, "Загрузка 3/4\nЧтение файлов", &mut loading_text);

    let mut app = match App::with_ui(ui, storage) {
        Ok(app) => app,
        Err(err) => {
            info!("{}", err);
            return Ok(());
        }
    };

    let wifi_connection = connect_wifi_networks(modem, &app.assets().config.networks);

    show_loading_stage(
        app.ui_mut(),
        "Загрузка 4/4\nПодключение WiFi",
        &mut loading_text,
    );

    let status_text = match wifi_connection.as_ref() {
        Some(conn) => alloc::format!("Готово\nWiFi: {}\nIP: {}", conn.ssid, conn.ip),
        None => "Готово\nWiFi: нет сети\nIP: --".to_string(),
    };
    render_text_screen(app.ui_mut(), &status_text);

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

    let touch = Gt911::new(i2c, touch_address);
    app = app.with_touch(touch);

    let touch_int = match PinDriver::input(gpios.gpio48) {
        Ok(d) => d,
        Err(err) => {
            info!("Touch INT pin error: {}", err);
            return Ok(());
        }
    };

    let mut idle_ms: u32 = 0;
    let mut sleep_shown = false;

    loop {
        if touch_int.is_low() {
            for event in app.poll_touch() {
                idle_ms = 0;
                sleep_shown = false;
                if !app.handle_event(event.clone()) {
                    let text = match event {
                        UiEvent::Tap(p) => alloc::format!("Touch\nX: {}\nY: {}", p.x, p.y),
                        UiEvent::Slide { from, to } => alloc::format!(
                            "Slide\n{}:{} -> {}:{}",
                            from.x, from.y, to.x, to.y
                        ),
                    };
                    render_text_screen(app.ui_mut(), &text);
                }
            }
        } else {
            app.release_touch();
            idle_ms = idle_ms.saturating_add(50);
            if !sleep_shown && idle_ms >= SLEEP_IDLE_MS {
                sleep_shown = true;
                match app.assets().sleep_image.clone() {
                    Some(img) => render_image_screen(app.ui_mut(), img),
                    None => {
                        info!("Sleep image not found");
                        render_text_screen(app.ui_mut(), "Экран выкл.");
                    }
                }
            }
        }

        FreeRtos::delay_ms(50);
    }
}
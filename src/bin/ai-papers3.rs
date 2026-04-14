#![no_std]
extern crate alloc;

use ai_papers3::{
    Clock, Config, Gt911, MainAppError, TouchEvent, TouchTracker, connect_wifi_networks,
    display_begin, display_commit, display_draw_text, load_image, read_file,
    set_display_font_size, set_display_rotation,
};
use alloc::string::ToString;
use embedded_sdmmc::{SdCard, VolumeManager};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config};
use esp_idf_hal::units::{KiloHertz, MegaHertz};
use log::info;

const DISPLAY_WIDTH: i32 = 960;
const DISPLAY_HEIGHT: i32 = 540;
const SLEEP_IDLE_MS: u32 = 30_000;

fn render_status(text: &str) {
    if let Err(err) = display_begin() {
        info!("Display init error: {}", err);
        return;
    }

    if let Err(err) = display_draw_text(text, 16, 16) {
        info!("Display text error: {}", err);
        return;
    }

    if let Err(err) = display_commit() {
        info!("Display commit error: {}", err);
    }
}

fn render_sleep_image(image: &ai_papers3::BmpImage) {
    if let Err(err) = display_begin() {
        info!("Display init error: {}", err);
        return;
    }

    let image_width = image.width as i32;
    let image_height = image.height as i32;
    let x = (DISPLAY_WIDTH - image_width) / 2;
    let y = (DISPLAY_HEIGHT - image_height) / 2;

    if let Err(err) = display_draw_bitmap(image, x, y) {
        info!("Display bitmap error: {}", err);
        return;
    }

    if let Err(err) = display_commit() {
        info!("Display commit error: {}", err);
    }
}

fn show_loading_stage(stage: &str, accumulated: &mut alloc::string::String) {
    info!("{}", stage);
    if !accumulated.is_empty() {
        accumulated.push('\n');
    }
    accumulated.push_str(stage);

    render_status(accumulated.as_str());
}

fn main() -> Result<(), MainAppError> {
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let mut loading_text = alloc::string::String::new();

    if let Err(err) = set_display_rotation(270) {
        info!("Display rotation error: {}", err);
    }

    if let Err(err) = set_display_font_size(18) {
        info!("Display font size error: {}", err);
    }

    show_loading_stage("Загрузка 1/4\nИнициализация SPI", &mut loading_text);

    let peripherals = peripherals::Peripherals::take().unwrap();
    let modem = peripherals.modem;
    let gpios = peripherals.pins;

    // Initialize SPI interface
    let spi = peripherals.spi2;
    let driver = match SpiDriver::new::<SPI2>(
        spi,
        gpios.gpio39,
        gpios.gpio38,
        Some(gpios.gpio40),
        &SpiDriverConfig::new(),
    ) {
        Ok(data) => data,
        Err(e) => {
            info!("SPI Driver initialization error: {}", e);
            return Ok(());
        }
    };

    let spi_device_config = config::Config::new().baudrate(MegaHertz(10).into());
    let spi_device = match SpiDeviceDriver::new(driver, Some(gpios.gpio47), &spi_device_config) {
        Ok(data) => data,
        Err(e) => {
            info!("SPI Device Driver initialization error: {}", e);
            return Ok(());
        }
    };

    let sdcard = SdCard::new(spi_device, esp_idf_hal::delay::FreeRtos);

    info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    show_loading_stage("Загрузка 2/4\nИнициализация SD", &mut loading_text);

    let volume_mgr = VolumeManager::new(sdcard, Clock);

    let volume0 = match volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)) {
        Ok(data) => data,
        Err(e) => {
            info!("Volume 0 open error: {:?}", e);
            return Ok(());
        }
    };

    info!("Volume 0: {:?}", volume0);

    show_loading_stage("Загрузка 3/4\nЧтение файлов", &mut loading_text);

    let root_dir = match volume0.open_root_dir() {
        Ok(data) => data,
        Err(e) => {
            info!("Root dir open error: {:?}", e);
            return Ok(());
        }
    };

    // let f = root_dir.open_file_in_dir(FILE_TO_READ, Mode::ReadOnly)?;

    root_dir
        .iterate_dir(|entry| {
            info!(
                "{:12} {:9} {} {}",
                entry.name,
                entry.size,
                entry.mtime,
                if entry.attributes.is_directory() {
                    "<DIR>"
                } else {
                    ""
                }
            );
        })
        .unwrap();

    let config = match read_file(&root_dir, "PAPERS~4.YAM") {
        Some(file_data) => match serde_yaml::from_slice::<Config>(&file_data) {
            Ok(data) => data,
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

    let sleep_image = load_image(&root_dir);

    show_loading_stage("Загрузка 4/4\nПодключение WiFi", &mut loading_text);

    let wifi_connection = connect_wifi_networks(modem, &config.networks);

    let status_text = match wifi_connection.as_ref() {
        Some(connection) => {
            alloc::format!("Готово\nWiFi: {}\nIP: {}", connection.ssid, connection.ip)
        }
        None => "Готово\nWiFi: нет сети\nIP: --".to_string(),
    };

    render_status(status_text.as_str());

    info!("Hello, world!");

    let i2c_config = I2cConfig::new().baudrate(KiloHertz(400).into());
    let mut i2c = match I2cDriver::new(peripherals.i2c0, gpios.gpio41, gpios.gpio42, &i2c_config) {
        Ok(data) => data,
        Err(err) => {
            info!("I2C init error: {}", err);
            return Ok(());
        }
    };

    let touch_address = match Gt911::detect_address(&mut i2c) {
        Some(address) => address,
        None => {
            info!("GT911 not found on I2C bus");
            return Ok(());
        }
    };

    let mut touch = Gt911::new(i2c, touch_address);
    let mut touch_tracker = TouchTracker::new(6);
    let mut idle_ms: u32 = 0;
    let mut sleep_shown = false;

    let touch_int = match PinDriver::input(gpios.gpio48) {
        Ok(data) => data,
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
                        match event {
                            TouchEvent::Touch(point) => {
                                let text = alloc::format!("Touch:\nX: {}\nY: {}", point.x, point.y);
                                render_status(text.as_str());
                            }
                            TouchEvent::Slide { from, to } => {
                                let text = alloc::format!(
                                    "Slide:\n{}:{} -> {}:{}",
                                    from.x,
                                    from.y,
                                    to.x,
                                    to.y
                                );
                                render_status(text.as_str());
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
                if let Some(image) = sleep_image.as_ref() {
                    render_sleep_image(image);
                    sleep_shown = true;
                } else {
                    info!("Sleep image OUTPUT.PNG not found");
                    sleep_shown = true;
                }
            }
        }

        FreeRtos::delay_ms(50);
    }
}

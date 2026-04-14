#![no_std]
extern crate alloc;

use ai_papers3::{
    Clock, Config, MainAppError, connect_wifi_networks, load_image, read_file,
    set_display_font_size, set_display_rotation, show_scene,
};
use embedded_sdmmc::{SdCard, VolumeManager};
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{SPI2, SpiDeviceDriver, SpiDriver, SpiDriverConfig, config};
use esp_idf_hal::units::MegaHertz;
use log::info;

fn show_loading_stage(stage: &str, accumulated: &mut alloc::string::String) {
    info!("{}", stage);
    if !accumulated.is_empty() {
        accumulated.push('\n');
    }
    accumulated.push_str(stage);

    if let Err(err) = show_scene(accumulated.as_str(), None) {
        info!("Display error: {}", err);
    }
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

    let image = load_image(&root_dir);

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

    show_loading_stage("Загрузка 4/4\nПодключение WiFi", &mut loading_text);

    let wifi_connection = connect_wifi_networks(modem, &config.networks);

    let status_text = match wifi_connection.as_ref() {
        Some(connection) => {
            alloc::format!("Готово\nWiFi: {}\nIP: {}", connection.ssid, connection.ip)
        }
        None => alloc::format!("Готово\nWiFi: нет сети\nIP: --"),
    };

    if let Err(err) = show_scene(status_text.as_str(), image.as_ref()) {
        info!("Display error: {}", err);
    }

    info!("Hello, world!");

    Ok(())
}

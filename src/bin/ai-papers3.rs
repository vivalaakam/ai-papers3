#![no_std]
extern crate alloc;

use ai_papers3::{decode_bmp, decode_png, read_file, show_scene, BmpImage, Clock, Config};
use alloc::format;
use alloc::vec::Vec;
use embedded_sdmmc::{BlockDevice, Directory, Mode, SdCard, TimeSource, VolumeManager};
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{config, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2};
use esp_idf_hal::units::MegaHertz;
use log::info;

fn load_image<
    D: BlockDevice,
    T: TimeSource,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    root_dir: &Directory<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
) -> Option<BmpImage> {
    if let Some(png_data) = read_file(root_dir, "OUTPUT.PNG") {
        match decode_png(&png_data) {
            Ok(image) => {
                info!("Loaded OUTPUT.PNG: {}x{}", image.width, image.height);
                return Some(image);
            }
            Err(err) => info!("PNG decode error: {}", err),
        }
    }

    if let Some(bmp_data) = read_file(root_dir, "OUTPUT.BMP") {
        match decode_bmp(&bmp_data) {
            Ok(image) => {
                info!("Loaded OUTPUT.BMP: {}x{}", image.width, image.height);
                return Some(image);
            }
            Err(err) => info!("BMP decode error: {}", err),
        }
    }

    None
}

fn main() {
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = peripherals::Peripherals::take().unwrap();
    let gpios = peripherals.pins;

    // Initialize SPI interface
    let spi = peripherals.spi2;
    let driver = SpiDriver::new::<SPI2>(
        spi,
        gpios.gpio39,
        gpios.gpio38,
        Some(gpios.gpio40),
        &SpiDriverConfig::new(),
    )
    .unwrap();

    let spi_device_config = config::Config::new().baudrate(MegaHertz(10).into());
    let spi_device = SpiDeviceDriver::new(driver, Some(gpios.gpio47), &spi_device_config).unwrap();

    let sdcard = SdCard::new(spi_device, esp_idf_hal::delay::FreeRtos);

    info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    let volume_mgr = VolumeManager::new(sdcard, Clock);

    let volume0 = volume_mgr
        .open_volume(embedded_sdmmc::VolumeIdx(0))
        .unwrap();
    info!("Volume 0: {:?}", volume0);

    let root_dir = volume0.open_root_dir().unwrap();

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

    match read_file(&root_dir, "PAPERS~4.YAM") {
        Some(file_data) => {
            let config: Config = serde_yaml::from_slice(&file_data).unwrap();

            info!("Config: {:?}", config);

            let text = format!("ai-papers3\n{}\ntext + rect", config.backend_url);
            if let Err(err) = show_scene(&text, image.as_ref()) {
                info!("Display error: {}", err);
            }
        }
        None => {
            info!("PAPERS~4.YAM not found");

            if let Err(err) = show_scene("ai-papers3\nDisplay demo\ntext + rect", image.as_ref()) {
                info!("Display error: {}", err);
            }
        }
    }

    info!("Hello, world!");
}

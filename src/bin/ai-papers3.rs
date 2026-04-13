#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use embedded_sdmmc::{Mode, SdCard, VolumeManager};
use log::info;
use esp_idf_hal::peripherals;
use esp_idf_hal::spi::{SpiDriver, SpiDriverConfig, SPI2, config, SpiDeviceDriver};
use esp_idf_hal::units::MegaHertz;
use ai_papers3::{Clock, Config};

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

    let volume_mgr = VolumeManager::new(
        sdcard,
        Clock
    );

    let volume0 = volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)).unwrap();
    info!("Volume 0: {:?}", volume0);

    let root_dir = volume0.open_root_dir().unwrap();

    // let f = root_dir.open_file_in_dir(FILE_TO_READ, Mode::ReadOnly)?;

    root_dir.iterate_dir(|entry| {
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
    }).unwrap();

    let f = root_dir.open_file_in_dir("PAPERS~4.YAM", Mode::ReadOnly);

    match f {
        Ok(f) => {
            info!("File opened: {:?} - {}", f, f.length());

            let mut file_data = Vec::new();

            while !f.is_eof() {
                let mut buf = [0u8; 16];
                let read = f.read(&mut buf).unwrap();
                file_data.extend_from_slice(&buf[..read]);
            }

            info!("{}", core::str::from_utf8(&file_data).unwrap());


            let config: Config = serde_yaml::from_slice(&file_data).unwrap();

            info!("Config: {:?}", config);
        },
        Err(e) => {
            info!("Error: {:?}", e);
        }
    }

    info!("Hello, world!");
}

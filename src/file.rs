use embedded_sdmmc::{BlockDevice, Directory, Mode, TimeSource};
use alloc::vec::Vec;
use log::info;

pub fn read_file<
    D: BlockDevice,
    T: TimeSource,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    root_dir: &Directory<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
    name: &str,
) -> Option<Vec<u8>> {
    let file = match root_dir.open_file_in_dir(name, Mode::ReadOnly) {
        Ok(file) => file,
        Err(err) => {
            info!("{} open error: {:?}", name, err);
            return None;
        }
    };

    let mut data = Vec::new();
    while !file.is_eof() {
        let mut buf = [0u8; 64];
        let read = file.read(&mut buf).unwrap();
        data.extend_from_slice(&buf[..read]);
    }

    Some(data)
}
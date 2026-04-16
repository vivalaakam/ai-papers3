use alloc::vec::Vec;
use embedded_sdmmc::{BlockDevice, Mode, TimeSource, VolumeIdx, VolumeManager};
use log::info;

use super::Storage;

pub struct SDCardStorage<
    D,
    T,
    const MAX_DIRS: usize = 4,
    const MAX_FILES: usize = 4,
    const MAX_VOLUMES: usize = 1,
> where
    D: BlockDevice,
    T: TimeSource,
    <D as BlockDevice>::Error: core::fmt::Debug,
{
    volume_mgr: VolumeManager<D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
    folder: Option<&'static str>,
}

impl<D, T, const MAX_DIRS: usize, const MAX_FILES: usize, const MAX_VOLUMES: usize>
    SDCardStorage<D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>
where
    D: BlockDevice,
    T: TimeSource,
    <D as BlockDevice>::Error: core::fmt::Debug,
{
    pub fn new(
        volume_mgr: VolumeManager<D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
        folder: Option<&'static str>,
    ) -> Self {
        Self { volume_mgr, folder }
    }
}

impl<D, T, const MAX_DIRS: usize, const MAX_FILES: usize, const MAX_VOLUMES: usize> Storage
    for SDCardStorage<D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>
where
    D: BlockDevice,
    T: TimeSource,
    <D as BlockDevice>::Error: core::fmt::Debug,
{
    fn read_file(&self, name: &str) -> Option<Vec<u8>> {
        let volume = self.volume_mgr.open_volume(VolumeIdx(0)).ok()?;
        let mut dir = volume.open_root_dir().ok()?;
        if let Some(folder) = self.folder {
            dir.change_dir(folder).ok()?;
        }

        let file = match dir.open_file_in_dir(name, Mode::ReadOnly) {
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
}

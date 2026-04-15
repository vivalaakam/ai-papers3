use alloc::vec::Vec;
use log::info;

pub trait Storage {
    fn read_file(&self, name: &str) -> Option<Vec<u8>>;
}

#[cfg(target_os = "espidf")]
mod embedded_impl {
    use super::Storage;
    use alloc::vec::Vec;
    use embedded_sdmmc::{BlockDevice, Directory, Mode, TimeSource};
    use log::info;

    impl<
            D: BlockDevice,
            T: TimeSource,
            const MAX_DIRS: usize,
            const MAX_FILES: usize,
            const MAX_VOLUMES: usize,
        > Storage for Directory<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>
    {
        fn read_file(&self, name: &str) -> Option<Vec<u8>> {
            let file = match self.open_file_in_dir(name, Mode::ReadOnly) {
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
}

#[cfg(feature = "simulator")]
pub struct LocalStorage {
    root: std::path::PathBuf,
}

#[cfg(feature = "simulator")]
impl LocalStorage {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

#[cfg(feature = "simulator")]
impl Storage for LocalStorage {
    fn read_file(&self, name: &str) -> Option<Vec<u8>> {
        let path = self.root.join(name);
        match std::fs::read(&path) {
            Ok(data) => Some(data),
            Err(err) => {
                info!("{} read error: {}", path.display(), err);
                None
            }
        }
    }
}

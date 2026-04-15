use alloc::vec::Vec;
use log::info;

use super::Storage;

pub struct LocalStorage {
    root: std::path::PathBuf,
}

impl LocalStorage {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

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

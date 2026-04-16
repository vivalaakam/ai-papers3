use alloc::vec::Vec;

pub trait Storage {
    fn read_file(&self, name: &str) -> Option<Vec<u8>>;
}

#[cfg(feature = "simulator")]
mod local;

#[cfg(feature = "simulator")]
pub use local::LocalStorage;

#[cfg(target_os = "espidf")]
mod sdcard;

#[cfg(target_os = "espidf")]
pub use sdcard::SDCardStorage;

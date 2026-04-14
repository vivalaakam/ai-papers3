use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub backend_url: String,
    pub networks: Vec<WifiNetwork>,
    pub display_rotation_degrees: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub password: String,
}

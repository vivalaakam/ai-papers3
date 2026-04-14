use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub backend_url: String,
    pub networks: Vec<WifiNetwork>,
}

#[derive(Debug, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub password: String,
}

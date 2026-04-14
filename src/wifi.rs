extern crate alloc;

use alloc::string::String;
use core::convert::TryInto;
use core::net::Ipv4Addr;

use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::info;

use crate::config::WifiNetwork;

pub struct WifiConnection {
    pub wifi: BlockingWifi<EspWifi<'static>>,
    pub ip: Ipv4Addr,
    pub ssid: String,
}

pub fn connect_wifi_networks<M: WifiModemPeripheral>(
    modem: impl Peripheral<P = M> + 'static,
    networks: &[WifiNetwork],
) -> Option<WifiConnection> {
    if networks.is_empty() {
        info!("Wifi config contains no networks");
        return None;
    }

    let sys_loop = match EspSystemEventLoop::take() {
        Ok(loop_handle) => loop_handle,
        Err(err) => {
            info!("Wifi event loop error: {}", err);
            return None;
        }
    };

    let nvs = match EspDefaultNvsPartition::take() {
        Ok(partition) => partition,
        Err(err) => {
            info!("Wifi NVS error: {}", err);
            return None;
        }
    };

    let mut wifi = match BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs)).ok()?,
        sys_loop,
    ) {
        Ok(wifi) => wifi,
        Err(err) => {
            info!("Wifi init error: {}", err);
            return None;
        }
    };

    for network in networks {
        let configuration = match build_client_configuration(network) {
            Some(configuration) => configuration,
            None => continue,
        };

        match try_connect(&mut wifi, network, &configuration) {
            Ok(ip) => {
                return Some(WifiConnection {
                    wifi,
                    ip,
                    ssid: network.ssid.clone(),
                });
            }
            Err(err) => {
                info!("Wifi connect failed for {}: {}", network.ssid, err);
                let _ = wifi.disconnect();
                let _ = wifi.stop();
            }
        }
    }

    None
}

fn build_client_configuration(network: &WifiNetwork) -> Option<Configuration> {
    let ssid = match network.ssid.as_str().try_into() {
        Ok(ssid) => ssid,
        Err(_) => {
            info!("Wifi SSID too long: {}", network.ssid);
            return None;
        }
    };

    let password = if network.password.is_empty() {
        heapless::String::new()
    } else {
        match network.password.as_str().try_into() {
            Ok(password) => password,
            Err(_) => {
                info!("Wifi password too long for {}", network.ssid);
                return None;
            }
        }
    };

    let auth_method = if network.password.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };

    Some(Configuration::Client(ClientConfiguration {
        ssid,
        bssid: None,
        auth_method,
        password,
        channel: None,
        ..Default::default()
    }))
}

fn try_connect(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    network: &WifiNetwork,
    configuration: &Configuration,
) -> Result<Ipv4Addr, String> {
    wifi.set_configuration(configuration)
        .map_err(|err| format!("config error: {}", err))?;

    wifi.start()
        .map_err(|err| format!("start error: {}", err))?;

    info!("Wifi started: {}", network.ssid);

    wifi.connect()
        .map_err(|err| format!("connect error: {}", err))?;

    info!("Wifi connected: {}", network.ssid);

    wifi.wait_netif_up()
        .map_err(|err| format!("netif error: {}", err))?;

    info!("Wifi netif up: {}", network.ssid);

    wifi.wifi()
        .sta_netif()
        .get_ip_info()
        .map(|info| info.ip)
        .map_err(|err| format!("ip error: {}", err))
}

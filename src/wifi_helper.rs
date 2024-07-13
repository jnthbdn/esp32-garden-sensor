use esp_idf_hal::sys::esp_wifi_set_country;
use esp_idf_hal::{modem::Modem, peripheral::Peripheral, sys::wifi_country_t};
use esp_idf_svc::hal::sys::esp;
use esp_idf_svc::wifi::AccessPointConfiguration;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    ipv4::{self, Mask, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDriver},
};
use log::info;
use std::{net::Ipv4Addr, str::FromStr};

use crate::nvs_configuration::NvsConfiguration;

pub fn connect_wifi<'a>(
    config: &NvsConfiguration,
    modem: impl Peripheral<P = Modem> + 'a,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: config.get_ssid().as_str().try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: config.get_passphrase().as_str().try_into().unwrap(),
        channel: None,
    });

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(wifi)
}

pub fn create_ap<'a>(
    modem: impl Peripheral<P = Modem> + 'a,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_drv = WifiDriver::new(modem, sys_loop.clone(), Some(nvs))?;
    let wifi_esp = EspWifi::wrap_all(
        wifi_drv,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: ipv4::Configuration::Router(ipv4::RouterConfiguration {
                subnet: Subnet {
                    gateway: Ipv4Addr::from_str("192.168.70.1")?,
                    mask: Mask(24),
                },
                ..Default::default()
            }),
            ..NetifConfiguration::wifi_default_router()
        })?,
    )?;

    let mut wifi = BlockingWifi::wrap(wifi_esp, sys_loop)?;

    let cc = wifi_country_t {
        cc: [b'F' as i8, b'R' as i8, 0 as i8],
        schan: 1,
        nchan: 14,
        max_tx_power: 80,
        ..Default::default()
    };

    esp!(unsafe { esp_wifi_set_country(&cc) })?;

    let wifi_configuration = Configuration::Mixed(
        ClientConfiguration {
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "ESP Config".try_into().unwrap(),
            ssid_hidden: false,
            auth_method: AuthMethod::None,
            max_connections: 5,
            channel: 11,
            ..Default::default()
        },
    );
    // let wifi_configuration = Configuration::AccessPoint(AccessPointConfiguration {
    //     ssid: "ESP Config".try_into().unwrap(),
    //     ssid_hidden: false,
    //     auth_method: AuthMethod::None,
    //     max_connections: 5,
    //     channel: 11,
    //     ..Default::default()
    // });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    // wifi.wait_netif_up()?;

    Ok(wifi)
}

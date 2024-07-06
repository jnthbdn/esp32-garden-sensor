use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Mutex;

use adc_helper::AdcHelper;
use anyhow::Ok;
use esp_idf_hal::adc::{attenuation, Adc};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{self, ADCPin, Output, PinDriver};
use esp_idf_hal::io::Write;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::sys::{adc_atten_t, esp_deep_sleep, esp_wifi_set_country, wifi_country_t};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::sys::esp;
use esp_idf_svc::http::{self, server::EspHttpServer, Method};
use esp_idf_svc::ipv4::{self, Mask, Subnet};
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AuthMethod, BlockingWifi, ClientConfiguration, Configuration,
    EspWifi, WifiDriver,
};
use log::info;
use main_configuration::MainConfiguration;
use moisture_sensor::MoistureSensor;
use post_data::PostData;

mod adc_helper;
mod main_configuration;
mod moisture_sensor;
mod post_data;
mod string_error;

// const V_HIGH_MOIST: f32 = 1.26;
// const V_LOW_MOIST: f32 = 2.55;

const MIN_BAT_VOLT: f32 = 3.2;
const MAX_BAT_VOLT: f32 = 4.2;

static INDEX_HTML: &str = include_str!("settings_page.html");

struct OnBoardLed<'a> {
    orange: PinDriver<'a, gpio::Gpio18, Output>,
    white: PinDriver<'a, gpio::Gpio19, Output>,
}

fn template_index(main_config: &MainConfiguration, error_message: Option<String>) -> String {
    let mut template = INDEX_HTML.to_string();

    template = template.replace("{SSID}", &main_config.get_ssid());
    template = template.replace("{NAME}", &main_config.get_name());
    template = template.replace("{ID}", &format!("{}", &main_config.get_id()));
    template = template.replace(
        "{VHIGH_MOIST}",
        &format!("{}", &main_config.get_vhigh_moisture()),
    );
    template = template.replace(
        "{VLOW_MOIST}",
        &format!("{}", &main_config.get_vlow_moisture()),
    );
    template = template.replace("{ERROR_MSG}", &error_message.unwrap_or("".to_string()));

    template
}

fn get_batt_percentage(voltage: f32) -> f32 {
    if voltage >= MAX_BAT_VOLT {
        return 100.0;
    } else if voltage <= MIN_BAT_VOLT {
        return 0.0;
    }

    let slope: f32 = 100.0 / (MAX_BAT_VOLT - MIN_BAT_VOLT);
    slope * (voltage - MIN_BAT_VOLT)
}

fn connect_wifi<'a>(
    config: &MainConfiguration,
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

fn create_ap<'a>(
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

    let wifi_configuration = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "ESP Config".try_into().unwrap(),
        ssid_hidden: false,
        auth_method: AuthMethod::None,
        max_connections: 5,
        channel: 11,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    Ok(wifi)
}

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let main_config = MainConfiguration::new()?;

    let mut leds = OnBoardLed {
        orange: PinDriver::output(peripherals.pins.gpio18)?,
        white: PinDriver::output(peripherals.pins.gpio19)?,
    };

    let adc_helper: AdcHelper<{ attenuation::DB_11 }, _, _, _> = AdcHelper::new(
        peripherals.adc1,
        peripherals.pins.gpio3,
        peripherals.pins.gpio4,
    )?;

    let mut button_settings = PinDriver::input(peripherals.pins.gpio5)?;
    button_settings.set_pull(gpio::Pull::Up)?;

    leds.white.set_low()?;
    leds.orange.set_low()?;

    FreeRtos::delay_ms(1000);

    if button_settings.is_high() {
        let _wifi = connect_wifi(&main_config, peripherals.modem)?;
        main_sensor(leds, main_config, adc_helper)?;
    } else {
        let _wifi = create_ap(peripherals.modem)?;
        main_settings(main_config, leds)?
    }

    Ok(())
}

fn main_settings(main_config: MainConfiguration, mut leds: OnBoardLed) -> anyhow::Result<()> {
    leds.white.set_high()?;

    let mutex_config = Mutex::new(main_config);

    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(template_index(&mutex_config.lock().unwrap(), None).as_bytes())
            .map(|_| ())
    })?;

    server.fn_handler::<anyhow::Error, _>("/", Method::Post, |mut req| {
        let len_body = req
            .header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let error_message: String;

        if len_body == 0 {
            error_message = "Save error: No body or no content-length".to_string();
        } else if len_body >= 256 {
            error_message = "Save error: Content-length too long.".to_string();
        } else {
            let mut buffer = [0u8; 156];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_data =
                        PostData::from_string(String::from_utf8(buffer[0..bytes_read].to_vec())?);
                    let mut mainconfig_lock = mutex_config.lock().unwrap();

                    if post_data.is_key_exists("ssid") {
                        mainconfig_lock.set_ssid(post_data.read_value("ssid").unwrap().as_str())?;
                    }

                    if post_data.is_key_exists("pass") {
                        mainconfig_lock
                            .set_passphrase(post_data.read_value("pass").unwrap().as_str())?;
                    }

                    if post_data.is_key_exists("name") {
                        mainconfig_lock.set_name(post_data.read_value("name").unwrap().as_str())?;
                    }

                    if post_data.is_key_exists("id") {
                        mainconfig_lock.set_id(
                            u32::from_str_radix(post_data.read_value("id").unwrap().as_str(), 10)
                                .unwrap_or(0),
                        )?;
                    }

                    if post_data.is_key_exists("vhigh_moist") {
                        mainconfig_lock.set_vhigh_moisture(
                            f32::from_str(post_data.read_value("vhigh_moist").unwrap().as_str())
                                .unwrap_or(0.0),
                        )?;
                    }

                    if post_data.is_key_exists("vlow_moist") {
                        mainconfig_lock.set_vlow_moisture(
                            f32::from_str(post_data.read_value("vlow_moist").unwrap().as_str())
                                .unwrap_or(0.0),
                        )?;
                    }

                    error_message = "Save successfully!".to_string();
                }
                Err(_) => {
                    error_message = "Save error: Failed to read request.".to_string();
                }
            };
        }

        req.into_ok_response()?.write_all(
            template_index(&mutex_config.lock().unwrap(), Some(error_message)).as_bytes(),
        )?;
        Ok(())
    })?;

    loop {
        FreeRtos::delay_ms(1);
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn main_sensor<'a, const A: adc_atten_t, ADC: Adc, P1: ADCPin<Adc = ADC>, P2: ADCPin<Adc = ADC>>(
    mut leds: OnBoardLed,
    main_config: MainConfiguration,
    mut adc_helper: AdcHelper<'a, A, ADC, P1, P2>,
) -> anyhow::Result<()> {
    leds.orange.set_high()?;

    FreeRtos::delay_ms(1000);

    let adc_batt_voltage = adc_helper.read_battery_value() as f32 / 1000.0;
    let adc_moisture_voltage = adc_helper.read_moisture_value() as f32 / 1000.0;

    let moisture_sensor = MoistureSensor::new(
        main_config.get_vhigh_moisture(),
        main_config.get_vlow_moisture(),
    );

    leds.orange.set_low()?;

    info!(
        "Battery Status: {:.2}% ({:.2} V)",
        get_batt_percentage(adc_batt_voltage * 2.0),
        adc_batt_voltage * 2.0
    );

    info!(
        "Moisture level: {:.2}% ({:.2} V)",
        moisture_sensor.get_moisture_level(adc_moisture_voltage),
        adc_moisture_voltage
    );

    info!("Going to sleep !");

    unsafe {
        esp_deep_sleep(10_000_000);
    }

    #[allow(unreachable_code)]
    Ok(())
}

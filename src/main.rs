use std::str::from_utf8;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Ok;
// use board::board::Board;
// use board::on_board_led::OnBoardLed;
use configuration::{main_configuration, nvs_configuration::NvsConfiguration};
use embedded_svc::{
    http::client::{Client as HttpClient, Response},
    utils::io,
};
use enumset::enum_set;
use esp_idf_svc::hal::adc::oneshot::AdcDriver;
use esp_idf_svc::hal::adc::ADC1;
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::Output;
use esp_idf_svc::hal::gpio::Pin;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::io::Write;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::sys::esp_deep_sleep;
use esp_idf_svc::hal::task::watchdog::TWDTConfig;
use esp_idf_svc::hal::task::watchdog::TWDTDriver;
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::http::{self, server::EspHttpServer, Method};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::{error, info};
use sensors::battery_sensor::BatterySensor;
use sensors::sensor::Sensor;
use serde_json::json;
use serde_json::Map;
use url_encoded_data::UrlEncodedData;

#[allow(unused_imports)]
use sensors::hcsr04_sensor::HCSR04Sensor;
#[allow(unused_imports)]
use sensors::moisture_sensor::MoistureSensor;

mod sensors {
    pub mod aht10_sensor;
    pub mod battery_sensor;
    pub mod hcsr04_sensor;
    pub mod moisture_sensor;
    pub mod sensor;
}

mod configuration {
    pub mod main_configuration;
    pub mod nvs_configuration;
}

mod string_error;
mod template;
mod wifi_helper;

type SensorsVec = Vec<Box<dyn Sensor + Send>>;

static mut ADC_1: Option<AdcDriver<ADC1>> = None;

fn adc1_ref() -> &'static AdcDriver<'static, ADC1> {
    unsafe { ADC_1.as_ref().unwrap() }
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let main_config = NvsConfiguration::new()?;
    let pins = peripherals.pins;

    let mut led_orange = PinDriver::output(pins.gpio0)?;
    let mut led_green = PinDriver::output(pins.gpio1)?;
    let config_button = PinDriver::input(pins.gpio7)?;

    // let wd_config = TWDTConfig {
    //     duration: Duration::from_secs(2),
    //     panic_on_trigger: false,
    //     subscribed_idle_tasks: enum_set!(Core::Core0),
    // };

    // let mut wd_driver = TWDTDriver::new(peripherals.twdt, &wd_config)?;
    // let mut watchdog = wd_driver.watch_current_task()?;

    unsafe {
        ADC_1 = Some(AdcDriver::new(peripherals.adc1)?);
    }

    led_green.set_low()?;
    led_orange.set_low()?;

    // loop {
    //     if config_button.is_high() {
    //         led_green.set_high();
    //     } else {
    //         led_green.set_low();
    //     }
    // }

    let mut sensors: SensorsVec = Vec::new();

    sensors.push(Box::new(BatterySensor::new(pins.gpio3, adc1_ref())?));

    #[cfg(feature = "moisture-sensor")]
    sensors.push(Box::new(MoistureSensor::new(
        adc1_ref(),
        pins.gpio4,
        pins.gpio6,
        main_config.get_vlow_moisture(),
        main_config.get_vhigh_moisture(),
    )?));

    #[cfg(feature = "water-level-sensor")]
    sensors.push(Box::new(HCSR04Sensor::new(
        pins.gpio6,
        pins.gpio4,
        pins.gpio5,
        main_config.get_low_water_level(),
        main_config.get_high_water_level(),
    )?));

    FreeRtos::delay_ms(3000);

    if config_button.is_high() {
        let wifi = wifi_helper::connect_wifi(&main_config, peripherals.modem);

        if wifi.is_ok() {
            error!(
                "[MAIN SENSOR] {}",
                main_sensor(main_config, &mut led_green, sensors).unwrap_err()
            );
        } else {
            error!("[WIFI] {}", wifi.err().unwrap());
        }

        error!("Retry connection in 5 seconds...");
        let start = SystemTime::now();
        loop {
            led_green.set_high()?;
            FreeRtos::delay_ms(100);
            led_green.set_low()?;
            FreeRtos::delay_ms(100);

            if start.elapsed().unwrap().as_secs() >= 5 {
                esp_idf_svc::hal::reset::restart();
            }
        }
    } else {
        let wifi = wifi_helper::create_ap(peripherals.modem);

        if wifi.is_ok() {
            error!(
                "[MAIN SETTINGS] {}",
                main_settings(main_config, wifi.unwrap(), &mut led_orange, sensors).unwrap_err()
            );
        } else {
            error!("[WIFI] {}", wifi.err().unwrap());
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn generate_json(sensors: &mut SensorsVec, main_config: &NvsConfiguration) -> serde_json::Value {
    let mut map = Map::new();

    map.insert("id".to_string(), json!(main_config.get_id()));
    map.insert("name".to_string(), json!(main_config.get_name()));

    for sensor in sensors {
        sensor.add_json_value(&mut map);
    }

    serde_json::Value::Object(map)
}

fn generate_html_value(sensors: &mut SensorsVec) -> String {
    let mut result = String::new();

    for sensor in sensors {
        result += &sensor.pretty_print();
        result += "\n";
    }

    result
}

fn main_settings<LedO: Pin>(
    main_config: NvsConfiguration,
    wifi: BlockingWifi<EspWifi>,
    led_orange: &mut PinDriver<'_, LedO, Output>,
    mut sensors: SensorsVec,
) -> anyhow::Result<()> {
    led_orange.set_high()?;

    let mutex_config = Mutex::new(main_config);
    let mutex_wifi = Mutex::new(wifi);
    let mutex_sensor = Mutex::new(sensors);

    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(
                template::to_html(
                    &mutex_config.lock().unwrap(),
                    None,
                    mutex_wifi.lock().unwrap().scan().ok(),
                    &generate_html_value(&mut mutex_sensor.lock().unwrap()), // &mutex_board.lock().unwrap().sensors.sensor_string_value(),
                )
                .as_bytes(),
            )
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
            let mut buffer = [0u8; 256];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_str = String::from_utf8(buffer[0..bytes_read].to_vec())?;
                    let post_data = UrlEncodedData::parse_str(&post_str);

                    let mut mainconfig_lock = mutex_config.lock().unwrap();

                    for elem in main_configuration::MAP_NVS_FORM {
                        if post_data.exists(elem.form_name) {
                            let data = post_data.get_first(&elem.form_name).unwrap();

                            match elem.data_type {
                                main_configuration::MapFormType::String(_, max_size) => {
                                    mainconfig_lock.store_string(&elem.nvs_key, data, max_size)?
                                }

                                main_configuration::MapFormType::Float(_) => mainconfig_lock
                                    .store_float(&elem.nvs_key, f32::from_str(data).unwrap())?,

                                main_configuration::MapFormType::U32Hex(_) => mainconfig_lock
                                    .store_u32(
                                        &elem.nvs_key,
                                        u32::from_str_radix(data, 16).unwrap(),
                                    )?,

                                main_configuration::MapFormType::Unsigned64(_) => {
                                    mainconfig_lock
                                        .store_u64(&elem.nvs_key, u64::from_str(data).unwrap())?
                                }

                                main_configuration::MapFormType::Unsigned8(_) => mainconfig_lock
                                    .store_u8(&elem.nvs_key, u8::from_str(data).unwrap())?,
                            };
                        }
                    }
                    error_message = "Save successfully!".to_string();
                }
                Err(_) => {
                    error_message = "Save error: Failed to read request.".to_string();
                }
            };
        }

        req.into_ok_response()?.write_all(
            template::to_html(
                &mutex_config.lock().unwrap(),
                Some(error_message),
                mutex_wifi.lock().unwrap().scan().ok(),
                &generate_html_value(&mut mutex_sensor.lock().unwrap()), // &mutex_board.lock().unwrap().sensors.sensor_string_value(),
            )
            .as_bytes(),
        )?;
        Ok(())
    })?;

    loop {}

    #[allow(unreachable_code)]
    Ok(())
}

fn main_sensor<LedG: Pin>(
    main_config: NvsConfiguration,
    led_green: &mut PinDriver<'_, LedG, Output>,
    mut sensors: SensorsVec,
) -> anyhow::Result<()> {
    led_green.set_high()?;

    FreeRtos::delay_ms(500);

    let url = main_configuration::make_http_url(&main_config);
    let payload_json = generate_json(&mut sensors, &main_config).to_string();

    info!("Send data to: '{}'", url);
    info!("JSON DATA: {}", payload_json);

    let mut client: HttpClient<EspHttpConnection> =
        HttpClient::wrap(EspHttpConnection::new(&Default::default())?);

    let headers = [
        ("content-type", "application/json"),
        ("content-length", &format!("{}", payload_json.len())),
    ];

    for attempt in 1..=5 {
        info!("Send data to server (attempt {}/5)", attempt);

        let mut request = match client.post(&url, &headers) {
            Result::Ok(req) => req,
            Err(e) => {
                log::warn!("Fail to create post: {}", e);
                continue;
            }
        };
        request.write_all(payload_json.as_bytes())?;
        request.flush()?;

        match request.submit() {
            Result::Ok(mut response) => {
                info!(
                    "Server response:\n\tStatus: {}\n\tBody: {}",
                    response.status(),
                    extract_data_or(&mut response)
                );
                break;
            }
            Err(error) => log::warn!("Failed to send data to server:\n\t{}", error),
        }
    }

    info!("Going to sleep !");
    led_green.set_low()?;

    unsafe {
        esp_deep_sleep(main_config.get_deep_sleep_duration());
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn extract_data_or(response: &mut Response<&mut EspHttpConnection>) -> String {
    let mut buff = [0u8; 1024];

    let byte_read = match io::try_read_full(response, &mut buff) {
        Result::Ok(len) => len,
        Err(e) => return format!("Fail read body ({}).", e.0),
    };

    match from_utf8(&buff[0..byte_read]) {
        Result::Ok(s) => s.to_string(),
        Err(e) => format!("Error decoding response body: {}", e),
    }
}

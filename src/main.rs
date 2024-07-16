use std::str::FromStr;
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::Ok;
use board::board::Board;
use configuration::{main_configuration, nvs_configuration::NvsConfiguration};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::sys::esp_deep_sleep;
use esp_idf_svc::http::{self, server::EspHttpServer, Method};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::{error, info};
use post_data::PostData;

mod board {
    pub mod board;
    pub mod buttons;
    pub mod on_board_led;
    pub mod sensors;
}
mod sensors {
    pub mod battery_sensor;
    pub mod moisture_sensor;
}
mod configuration {
    pub mod main_configuration;
    pub mod nvs_configuration;
}
mod post_data;
mod string_error;
mod template;
mod wifi_helper;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let main_config = NvsConfiguration::new()?;

    let mut board = Board::new(&main_config, peripherals.adc1, peripherals.pins)?;

    board.leds.green.set_low()?;
    board.leds.orange.set_low()?;

    FreeRtos::delay_ms(1000);

    if board.buttons.settings.is_high() {
        let wifi = wifi_helper::connect_wifi(&main_config, peripherals.modem);

        if wifi.is_ok() {
            error!(
                "[MAIN SENSOR] {}",
                main_sensor(&mut board, main_config).unwrap_err()
            );
        } else {
            error!("[WIFI] {}", wifi.err().unwrap());
        }

        error!("Retry connection in 10 seconds...");
        let start = SystemTime::now();
        loop {
            board.leds.orange.set_high()?;
            FreeRtos::delay_ms(100);
            board.leds.orange.set_low()?;
            FreeRtos::delay_ms(100);

            if start.elapsed().unwrap().as_secs() >= 10 {
                esp_idf_hal::reset::restart();
            }
        }
    } else {
        let wifi = wifi_helper::create_ap(peripherals.modem);

        if wifi.is_ok() {
            error!(
                "[MAIN SETTINGS] {}",
                main_settings(main_config, &mut board, wifi.unwrap()).unwrap_err()
            );
        } else {
            error!("[WIFI] {}", wifi.err().unwrap());
        }

        loop {
            board.leds.green.set_high()?;
            FreeRtos::delay_ms(100);
            board.leds.green.set_low()?;
            FreeRtos::delay_ms(100);
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn main_settings(
    main_config: NvsConfiguration,
    board: &mut Board,
    wifi: BlockingWifi<EspWifi>,
) -> anyhow::Result<()> {
    board.leds.green.set_high()?;

    let mutex_config = Mutex::new(main_config);
    let mutex_wifi = Mutex::new(wifi);

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
            let mut buffer = [0u8; 156];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_data =
                        PostData::from_string(String::from_utf8(buffer[0..bytes_read].to_vec())?);
                    let mut mainconfig_lock = mutex_config.lock().unwrap();

                    for elem in main_configuration::MAP_NVS_FORM {
                        if post_data.is_key_exists(elem.form_name) {
                            let data = post_data.read_value(&elem.form_name).unwrap();

                            match elem.data_type {
                                main_configuration::MapFormType::String(_, max_size) => {
                                    mainconfig_lock.store_string(
                                        &elem.nvs_key,
                                        data.as_str(),
                                        max_size,
                                    )?
                                }

                                main_configuration::MapFormType::Float(_) => mainconfig_lock
                                    .store_float(
                                        &elem.nvs_key,
                                        f32::from_str(data.as_str()).unwrap(),
                                    )?,

                                main_configuration::MapFormType::U32Hex(_) => mainconfig_lock
                                    .store_u32(
                                        &elem.nvs_key,
                                        u32::from_str_radix(data.as_str(), 16).unwrap(),
                                    )?,

                                main_configuration::MapFormType::Unsigned64(_) => mainconfig_lock
                                    .store_u64(
                                    &elem.nvs_key,
                                    u64::from_str(data.as_str()).unwrap(),
                                )?,

                                main_configuration::MapFormType::Unsigned8(_) => mainconfig_lock
                                    .store_u8(
                                        &elem.nvs_key,
                                        u8::from_str(data.as_str()).unwrap(),
                                    )?,
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
            )
            .as_bytes(),
        )?;
        Ok(())
    })?;

    loop {
        FreeRtos::delay_ms(1);
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn main_sensor<'a>(board: &mut Board, main_config: NvsConfiguration) -> anyhow::Result<()> {
    board.leds.orange.set_high()?;

    FreeRtos::delay_ms(1000);

    board.leds.orange.set_low()?;

    info!(
        "Battery Status: {:.2}% ({:.2} V)",
        board.sensors.battery_sensor.get_level(),
        board.sensors.battery_sensor.read_raw_value() as f32 / 1000.0
    );

    info!(
        "Moisture level: {:.2}% ({:.2} V)",
        board.sensors.moisture_sensor.get_level(),
        board.sensors.moisture_sensor.read_raw_value() as f32 / 1000.0
    );

    info!("Going to sleep !");

    unsafe {
        esp_deep_sleep(main_config.get_deep_sleep_duration());
    }

    #[allow(unreachable_code)]
    Ok(())
}

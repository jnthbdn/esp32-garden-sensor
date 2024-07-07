use std::str::FromStr;
use std::sync::Mutex;

use adc_helper::AdcHelper;
use anyhow::Ok;
use esp_idf_hal::adc::{attenuation, Adc};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{self, ADCPin, Output, PinDriver};
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::sys::{adc_atten_t, esp_deep_sleep};
use esp_idf_svc::http::{self, server::EspHttpServer, Method};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::info;
use main_configuration::MainConfiguration;
use moisture_sensor::MoistureSensor;
use post_data::PostData;

mod adc_helper;
mod configuration;
mod main_configuration;
mod moisture_sensor;
mod post_data;
mod string_error;
mod template_helper;
mod wifi_helper;

// const V_HIGH_MOIST: f32 = 1.26;
// const V_LOW_MOIST: f32 = 2.55;

const MIN_BAT_VOLT: f32 = 3.2;
const MAX_BAT_VOLT: f32 = 4.2;

struct OnBoardLed<'a> {
    orange: PinDriver<'a, gpio::Gpio18, Output>,
    white: PinDriver<'a, gpio::Gpio19, Output>,
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
        let _wifi = wifi_helper::connect_wifi(&main_config, peripherals.modem)?;
        main_sensor(leds, main_config, adc_helper)?;
    } else {
        let wifi = wifi_helper::create_ap(peripherals.modem)?;
        main_settings(main_config, leds, wifi)?
    }

    Ok(())
}

fn main_settings(
    main_config: MainConfiguration,
    mut leds: OnBoardLed,
    wifi: BlockingWifi<EspWifi>,
) -> anyhow::Result<()> {
    leds.white.set_high()?;

    let mutex_config = Mutex::new(main_config);
    let mutex_wifi = Mutex::new(wifi);

    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    server.fn_handler("/", Method::Get, |req| {
        let scan_result = mutex_wifi.lock().unwrap().scan();

        info!("Scan result: {:?}", scan_result);

        req.into_ok_response()?
            .write_all(
                template_helper::template_moisture(&mutex_config.lock().unwrap(), None).as_bytes(),
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

                    for elem in configuration::MAP_NVS_FORM {
                        if post_data.is_key_exists(elem.form_name) {
                            let data = post_data.read_value(&elem.form_name).unwrap();

                            match elem.data_type {
                                configuration::MapFormType::String => {
                                    mainconfig_lock.store_string(&elem.nvs_key, data.as_str())?
                                }

                                configuration::MapFormType::Float => mainconfig_lock.store_float(
                                    &elem.nvs_key,
                                    f32::from_str(data.as_str()).unwrap(),
                                )?,

                                configuration::MapFormType::Unsigned => mainconfig_lock
                                    .store_unsigned(
                                        &elem.nvs_key,
                                        u32::from_str(data.as_str()).unwrap(),
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
            template_helper::template_moisture(&mutex_config.lock().unwrap(), Some(error_message))
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

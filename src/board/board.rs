use std::{cell::RefCell, rc::Rc};

use esp_idf_hal::{
    adc::{config::Config, AdcDriver, ADC1},
    gpio::*,
};
use serde_json::json;

use crate::{
    configuration::nvs_configuration::NvsConfiguration,
    sensors::{battery_sensor::BatterySensor, moisture_sensor::MoistureSensor},
};

use super::{buttons::Buttons, on_board_led::OnBoardLed, sensors::Sensors};

pub struct Board<'a> {
    pub sensors: Sensors<'a>,
    pub buttons: Buttons<'a>,
    pub leds: OnBoardLed<'a>,
}

impl<'a> Board<'a> {
    pub fn new(main_config: &NvsConfiguration, adc_1: ADC1, pins: Pins) -> anyhow::Result<Self> {
        let adc_refcell = Rc::new(RefCell::new(AdcDriver::new(
            adc_1,
            &Config::new().calibration(true),
        )?));

        let mut s = Self {
            sensors: Sensors {
                #[cfg(feature = "moisture-sensor")]
                moisture_sensor: MoistureSensor::new(
                    adc_refcell.clone(),
                    pins.gpio2,
                    pins.gpio4,
                    main_config.get_vhigh_moisture(),
                    main_config.get_vlow_moisture(),
                )?,
                battery_sensor: BatterySensor::new(adc_refcell.clone(), pins.gpio3)?,
            },
            buttons: Buttons {
                settings: PinDriver::input(pins.gpio5)?,
            },
            leds: OnBoardLed {
                orange: PinDriver::output(pins.gpio6)?,
                green: PinDriver::output(pins.gpio7)?,
            },
        };

        s.buttons.settings.set_pull(Pull::Up)?;
        Ok(s)
    }

    pub fn generate_json(&mut self, main_config: &NvsConfiguration) -> String {
        json!({
            "name": main_config.get_name(),
            "id": main_config.get_id(),
            "sensors": self.sensors.to_json()
        })
        .to_string()
    }
}

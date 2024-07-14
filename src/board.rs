use std::{cell::RefCell, rc::Rc};

use esp_idf_hal::{
    adc::{config::Config, AdcDriver, ADC1},
    gpio::*,
};

use crate::{
    configuration::nvs_configuration::NvsConfiguration,
    sensors::{battery_sensor::BatterySensor, moisture_sensor::MoistureSensor},
};

#[cfg(feature = "moisture-sensor")]
pub struct Sensors<'a> {
    pub moisture_sensor: MoistureSensor<'a, ADC1, Gpio2, Gpio4>,
    pub battery_sensor: BatterySensor<'a, ADC1, Gpio3>,
}

#[cfg(feature = "moisture-sensor")]
pub struct Buttons<'a> {
    pub settings: PinDriver<'a, Gpio5, Input>,
}

pub struct OnBoardLed<'a> {
    pub orange: PinDriver<'a, Gpio6, Output>,
    pub green: PinDriver<'a, Gpio7, Output>,
}

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

        #[cfg(feature = "moisture-sensor")]
        let mut s = Self {
            sensors: Sensors {
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
}

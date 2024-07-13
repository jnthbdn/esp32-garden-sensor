use esp_idf_hal::{
    adc::{attenuation, config::Config, AdcChannelDriver, AdcDriver, ADC1},
    gpio::*,
};

use crate::{moisture_sensor::MoistureSensor, nvs_configuration::NvsConfiguration};

const MIN_BAT_VOLT: f32 = 3.2;
const MAX_BAT_VOLT: f32 = 4.2;

#[cfg(feature = "moisture-sensor")]
pub struct AdcSensor<'a> {
    adc: AdcDriver<'a, ADC1>,
    adc_pin_moist: AdcChannelDriver<'a, { attenuation::DB_11 }, Gpio4>,

    moisture_sensor: MoistureSensor,
}

#[cfg(feature = "moisture-sensor")]
impl<'a> AdcSensor<'a> {
    pub fn read_raw_moisture_value(&mut self) -> u16 {
        self.adc.read(&mut self.adc_pin_moist).unwrap_or(0)
    }

    pub fn read_moisture_value(&mut self) -> f32 {
        self.moisture_sensor
            .get_moisture_level(self.adc.read(&mut self.adc_pin_moist).unwrap_or(0) as f32 / 1000.0)
    }
}

#[cfg(feature = "moisture-sensor")]
pub struct Buttons<'a> {
    pub settings: PinDriver<'a, Gpio5, Input>,
}

pub struct OnBoardLed<'a> {
    pub orange: PinDriver<'a, Gpio18, Output>,
    pub white: PinDriver<'a, Gpio19, Output>,
}

pub struct BatterySensor<'a> {
    adc_pin_bat: AdcChannelDriver<'a, { attenuation::DB_11 }, Gpio3>,
}

pub struct BoardHelper<'a> {
    pub adc: AdcSensor<'a>,
    pub battery: BatterySensor<'a>,
    pub buttons: Buttons<'a>,
    pub leds: OnBoardLed<'a>,
}

impl<'a> BoardHelper<'a> {
    pub fn new(main_config: &NvsConfiguration, adc_1: ADC1, pins: Pins) -> anyhow::Result<Self> {
        #[cfg(feature = "moisture-sensor")]
        let mut s = Self {
            adc: AdcSensor {
                adc: AdcDriver::new(adc_1, &Config::new().calibration(true))?,
                adc_pin_moist: AdcChannelDriver::new(pins.gpio4)?,
                moisture_sensor: MoistureSensor::new(
                    main_config.get_vhigh_moisture(),
                    main_config.get_vlow_moisture(),
                ),
            },
            battery: BatterySensor {
                adc_pin_bat: AdcChannelDriver::new(pins.gpio3)?,
            },
            buttons: Buttons {
                settings: PinDriver::input(pins.gpio5)?,
            },
            leds: OnBoardLed {
                orange: PinDriver::output(pins.gpio18)?,
                white: PinDriver::output(pins.gpio19)?,
            },
        };

        s.buttons.settings.set_pull(Pull::Up)?;
        Ok(s)
    }

    pub fn read_raw_battery_value(&mut self) -> u16 {
        self.adc
            .adc
            .read(&mut self.battery.adc_pin_bat)
            .unwrap_or(0)
    }

    pub fn read_battery_value(&mut self) -> f32 {
        let mut voltage = self
            .adc
            .adc
            .read(&mut self.battery.adc_pin_bat)
            .unwrap_or(0) as f32;
        voltage = voltage.min(0.0).max(100.0);

        let slope: f32 = 100.0 / (MAX_BAT_VOLT - MIN_BAT_VOLT);
        slope * (voltage - MIN_BAT_VOLT)
    }
}

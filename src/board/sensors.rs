use esp_idf_hal::{adc::ADC1, gpio::*};

use crate::sensors::{battery_sensor::BatterySensor, moisture_sensor::MoistureSensor};

pub struct Sensors<'a> {
    #[cfg(feature = "moisture-sensor")]
    pub moisture_sensor: MoistureSensor<'a, ADC1, Gpio2, Gpio4>,

    pub battery_sensor: BatterySensor<'a, ADC1, Gpio3>,
}

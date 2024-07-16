use esp_idf_hal::{adc::ADC1, gpio::*};
use serde_json::json;

use crate::sensors::{battery_sensor::BatterySensor, moisture_sensor::MoistureSensor};

pub struct Sensors<'a> {
    #[cfg(feature = "moisture-sensor")]
    pub moisture_sensor: MoistureSensor<'a, ADC1, Gpio2, Gpio4>,

    pub battery_sensor: BatterySensor<'a, ADC1, Gpio3>,
}

impl<'a> Sensors<'a> {
    pub fn to_json(&mut self) -> serde_json::Value {
        #[cfg(feature = "moisture-sensor")]
        json!( {
                "battery": self.battery_sensor.get_level(),
                "moisture": self.moisture_sensor.get_level(),
        })
    }
}

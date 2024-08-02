use esp_idf_svc::hal::{adc::ADC1, gpio::*};
use serde_json::{json, Map, Value};

use crate::sensors::{
    battery_sensor::BatterySensor, hcsr04_sensor::HCSR04Sensor, moisture_sensor::MoistureSensor,
};

pub struct Sensors<'a> {
    #[cfg(feature = "moisture-sensor")]
    pub moisture_sensor: MoistureSensor<'a, ADC1, Gpio2, Gpio4>,

    #[cfg(feature = "water-level-sensor")]
    pub water_level_sensor: HCSR04Sensor<'a, Gpio10, Gpio0, Gpio1>,

    pub battery_sensor: BatterySensor<'a, ADC1, Gpio3>,
}

impl<'a> Sensors<'a> {
    pub fn to_json(&mut self) -> serde_json::Value {
        let mut map = Map::new();

        map.insert(
            "battery".to_string(),
            json!(self.battery_sensor.get_level()),
        );

        #[cfg(feature = "moisture-sensor")]
        map.insert("level".to_string(), json!(self.moisture_sensor.get_level()));

        #[cfg(feature = "water-level-sensor")]
        {
            map.insert(
                "level".to_string(),
                json!(self.water_level_sensor.get_level()),
            );
            map.insert(
                "raw".to_string(),
                json!(self.water_level_sensor.get_distance_mm()),
            );
        }

        Value::Object(map)
    }
}

use std::borrow::Borrow;

use esp_idf_svc::hal::{
    adc::{
        attenuation::DB_11,
        oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        Adc,
    },
    gpio::ADCPin,
};
use serde_json::json;

use super::sensor::Sensor;

const MIN_BAT_VOLT: f32 = 3.2;
const MAX_BAT_VOLT: f32 = 4.2;

pub struct BatterySensor<'a, ADC: Adc + 'a, APin: ADCPin<Adc = ADC>, M: Borrow<AdcDriver<'a, ADC>>>
{
    channel: AdcChannelDriver<'a, APin, M>,
}

impl<'a, ADC: Adc + 'a, APin: ADCPin<Adc = ADC>, M: Borrow<AdcDriver<'a, ADC>>>
    BatterySensor<'a, ADC, APin, M>
{
    pub fn new(pin: APin, adc_driver: M) -> anyhow::Result<Self> {
        Ok(Self {
            channel: AdcChannelDriver::new(
                adc_driver,
                pin,
                &AdcChannelConfig {
                    attenuation: DB_11,
                    calibration: true,
                    ..Default::default()
                },
            )?,
        })
    }

    pub fn read_raw_value(&mut self, nb_sample: u8) -> u16 {
        let mut result: u16 = 0;
        for _ in 0..nb_sample {
            result += self.channel.read().unwrap_or(0);
        }

        result / nb_sample as u16
    }

    pub fn get_level(&mut self) -> f32 {
        let adc_value = self.read_raw_value(10) as f32 / 1000.0 * 2.0;

        let slope = 100.0 / (MAX_BAT_VOLT - MIN_BAT_VOLT);
        let level = slope * (adc_value - MIN_BAT_VOLT);

        level.clamp(0.0, 100.0)
    }
}

impl<'a, ADC: Adc + 'a, APin: ADCPin<Adc = ADC>, M: Borrow<AdcDriver<'a, ADC>>> Sensor
    for BatterySensor<'a, ADC, APin, M>
{
    fn add_json_value(&mut self, map: &mut serde_json::Map<String, serde_json::Value>) {
        map.insert("battery".to_string(), json!(self.get_level()));
    }

    fn pretty_print(&mut self) -> String {
        format!(
            "Battery level: {}% (voltage: {} V)",
            self.get_level(),
            self.read_raw_value(5) as f32 / 1000.0 * 2.0
        )
    }
}

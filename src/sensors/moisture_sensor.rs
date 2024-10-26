// use std::sync::{Arc, Mutex};

// use esp_idf_svc::hal::{
//     adc::{attenuation, Adc, AdcChannelDriver, AdcDriver},
//     delay::FreeRtos,
//     gpio::{ADCPin, Output, OutputPin, PinDriver},
// };

// use super::sensor::Sensor;

// pub struct MoistureSensor<'a, ADC: Adc, PEN: OutputPin, PADC: ADCPin<Adc = ADC>> {
//     adc_ref: Arc<Mutex<AdcDriver<'a, ADC>>>,
//     pin_adc: AdcChannelDriver<'a, { attenuation::DB_11 }, PADC>,
//     pin_enable: PinDriver<'a, PEN, Output>,
//     v_high: f32,
//     v_low: f32,
// }

// impl<'a, ADC: Adc, PEN: OutputPin, PADC: ADCPin<Adc = ADC> + 'a>
//     MoistureSensor<'a, ADC, PEN, PADC>
// {
//     pub fn new(
//         adc: Arc<Mutex<AdcDriver<'a, ADC>>>,
//         pin_enable: PEN,
//         pin_adc: PADC,
//         voltage_high: f32,
//         voltage_low: f32,
//     ) -> anyhow::Result<Self> {
//         let mut s = Self {
//             adc_ref: adc,
//             pin_adc: AdcChannelDriver::new(pin_adc)?,
//             pin_enable: PinDriver::output(pin_enable)?,
//             v_high: voltage_high,
//             v_low: voltage_low,
//         };

//         s.pin_enable.set_low()?;

//         Ok(s)
//     }

//     pub fn read_raw_value(&mut self) -> u16 {
//         let _ = self.pin_enable.set_high();
//         FreeRtos::delay_ms(100);

//         let value = self
//             .adc_ref
//             .lock()
//             .unwrap()
//             .read(&mut self.pin_adc)
//             .unwrap_or(0);

//         let _ = self.pin_enable.set_low();
//         FreeRtos::delay_ms(100);

//         return value;
//     }

//     pub fn get_level(&mut self) -> f32 {
//         let adc_value = self.read_raw_value() as f32 / 1000.0;
//         let slope: f32 = 100.0 / (self.v_high - self.v_low);
//         (slope * (adc_value - self.v_low)).clamp(0.0, 100.0)
//     }
// }

// impl<'a, ADC: Adc, PEN: OutputPin, PADC: ADCPin<Adc = ADC> + 'a> Sensor
//     for MoistureSensor<'a, ADC, PEN, PADC>
// {
// }

use std::borrow::Borrow;

use esp_idf_svc::hal::{
    adc::{
        attenuation,
        oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        Adc,
    },
    delay::FreeRtos,
    gpio::{ADCPin, Output, OutputPin, PinDriver},
};
use serde_json::json;

use super::sensor::Sensor;

pub struct MoistureSensor<
    'a,
    ADC: Adc + 'a,
    PEN: OutputPin,
    APin: ADCPin<Adc = ADC>,
    M: Borrow<AdcDriver<'a, ADC>>,
> {
    channel: AdcChannelDriver<'a, APin, M>,
    pin_enable: PinDriver<'a, PEN, Output>,
    v_high: f32,
    v_low: f32,
}

impl<'a, ADC: Adc + 'a, PEN: OutputPin, APin: ADCPin<Adc = ADC>, M: Borrow<AdcDriver<'a, ADC>>>
    MoistureSensor<'a, ADC, PEN, APin, M>
{
    pub fn new(
        adc_driver: M,
        pin_adc: APin,
        pin_enable: PEN,
        voltage_high: f32,
        voltage_low: f32,
    ) -> anyhow::Result<Self> {
        let mut s = Self {
            channel: AdcChannelDriver::new(
                adc_driver,
                pin_adc,
                &AdcChannelConfig {
                    attenuation: attenuation::DB_11,
                    calibration: true,
                    ..Default::default()
                },
            )?,
            pin_enable: PinDriver::output(pin_enable)?,
            v_high: voltage_high,
            v_low: voltage_low,
        };

        s.pin_enable.set_low()?;

        Ok(s)
    }

    pub fn read_raw_value(&mut self, nb_sample: u8) -> u16 {
        let _ = self.pin_enable.set_high();
        FreeRtos::delay_ms(100);

        let mut result = 0;

        for _ in 0..nb_sample {
            result += self.channel.read().unwrap_or(0);
        }

        let _ = self.pin_enable.set_low();
        FreeRtos::delay_ms(100);

        result / nb_sample as u16
    }

    pub fn get_level(&mut self) -> f32 {
        let adc_value = self.read_raw_value(10) as f32 / 1000.0;
        let slope: f32 = 100.0 / (self.v_high - self.v_low);
        (slope * (adc_value - self.v_low)).clamp(0.0, 100.0)
    }
}

impl<'a, ADC: Adc + 'a, PEN: OutputPin, APin: ADCPin<Adc = ADC>, M: Borrow<AdcDriver<'a, ADC>>>
    Sensor for MoistureSensor<'a, ADC, PEN, APin, M>
{
    fn add_json_value(&mut self, map: &mut serde_json::Map<String, serde_json::Value>) {
        map.insert("level".to_string(), json!(self.get_level()));
    }

    fn pretty_print(&mut self) -> String {
        format!(
            "Moisture level: {}% (raw value: {})",
            self.get_level(),
            self.read_raw_value(5)
        )
    }
}

use std::{cell::RefCell, rc::Rc};

use esp_idf_hal::{
    adc::{attenuation, Adc, AdcChannelDriver, AdcDriver},
    gpio::ADCPin,
};

const MIN_BAT_VOLT: f32 = 3.2;
const MAX_BAT_VOLT: f32 = 4.2;

pub struct BatterySensor<'a, ADC: Adc, PADC: ADCPin<Adc = ADC>> {
    adc_ref: Rc<RefCell<AdcDriver<'a, ADC>>>,
    pin_adc: AdcChannelDriver<'a, { attenuation::DB_11 }, PADC>,
}

impl<'a, ADC: Adc, PADC: ADCPin<Adc = ADC> + 'a> BatterySensor<'a, ADC, PADC> {
    pub fn new(adc: Rc<RefCell<AdcDriver<'a, ADC>>>, pin_adc: PADC) -> anyhow::Result<Self> {
        Ok(Self {
            adc_ref: adc,
            pin_adc: AdcChannelDriver::new(pin_adc)?,
        })
    }

    pub fn read_raw_value(&mut self) -> u16 {
        self.adc_ref
            .borrow_mut()
            .read(&mut self.pin_adc)
            .unwrap_or(0)
    }

    pub fn get_level(&mut self) -> f32 {
        let adc_value = self.read_raw_value() as f32 / 1000.0 * 2.0;

        let slope = 100.0 / (MAX_BAT_VOLT - MIN_BAT_VOLT);
        let level = slope * (adc_value - MIN_BAT_VOLT);

        level.clamp(0.0, 100.0)
    }
}

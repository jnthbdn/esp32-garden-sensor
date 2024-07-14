use std::{cell::RefCell, rc::Rc};

use esp_idf_hal::{
    adc::{attenuation, Adc, AdcChannelDriver, AdcDriver},
    delay::FreeRtos,
    gpio::{ADCPin, Output, OutputPin, PinDriver},
};

pub struct MoistureSensor<'a, ADC: Adc, PEN: OutputPin, PADC: ADCPin<Adc = ADC>> {
    adc_ref: Rc<RefCell<AdcDriver<'a, ADC>>>,
    pin_adc: AdcChannelDriver<'a, { attenuation::DB_11 }, PADC>,
    pin_enable: PinDriver<'a, PEN, Output>,
    v_high: f32,
    v_low: f32,
}

impl<'a, ADC: Adc, PEN: OutputPin, PADC: ADCPin<Adc = ADC> + 'a>
    MoistureSensor<'a, ADC, PEN, PADC>
{
    pub fn new(
        adc: Rc<RefCell<AdcDriver<'a, ADC>>>,
        pin_enable: PEN,
        pin_adc: PADC,
        voltage_high: f32,
        voltage_low: f32,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            adc_ref: adc,
            pin_adc: AdcChannelDriver::new(pin_adc)?,
            pin_enable: PinDriver::output(pin_enable)?,
            v_high: voltage_high,
            v_low: voltage_low,
        })
    }

    pub fn read_raw_value(&mut self) -> u16 {
        let _ = self.pin_enable.set_high();
        FreeRtos::delay_ms(100);

        let value = self
            .adc_ref
            .borrow_mut()
            .read(&mut self.pin_adc)
            .unwrap_or(0);

        let _ = self.pin_enable.set_low();
        FreeRtos::delay_ms(100);

        return value;
    }

    pub fn get_level(&mut self) -> f32 {
        let adc_value = self.read_raw_value() as f32 / 1000.0;
        let slope: f32 = 100.0 / (self.v_high - self.v_low);
        (slope * (adc_value - self.v_low)).clamp(0.0, 100.0)
    }
}

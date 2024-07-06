use esp_idf_hal::{
    adc::{config::Config, Adc, AdcChannelDriver, AdcDriver},
    gpio::ADCPin,
    peripheral::Peripheral,
    sys::adc_atten_t,
};

pub struct AdcHelper<
    'a,
    const A: adc_atten_t,
    ADC: Adc,
    P1: ADCPin<Adc = ADC>,
    P2: ADCPin<Adc = ADC>,
> {
    adc_drv: AdcDriver<'a, ADC>,
    adc_pin_bat: AdcChannelDriver<'a, A, P1>,
    adc_pin_moist: AdcChannelDriver<'a, A, P2>,
}

impl<'a, const A: adc_atten_t, ADC: Adc, P1: ADCPin<Adc = ADC>, P2: ADCPin<Adc = ADC>>
    AdcHelper<'a, A, ADC, P1, P2>
{
    pub fn new(
        adc: impl Peripheral<P = ADC> + 'a,
        pin_batt: impl Peripheral<P = P1> + 'a,
        pin_moist: impl Peripheral<P = P2> + 'a,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            adc_drv: AdcDriver::new(adc, &Config::new().calibration(true))?,
            adc_pin_bat: AdcChannelDriver::new(pin_batt)?,
            adc_pin_moist: AdcChannelDriver::new(pin_moist)?,
        })
    }

    pub fn read_battery_value(&mut self) -> u16 {
        self.adc_drv.read(&mut self.adc_pin_bat).unwrap_or(0)
    }

    pub fn read_moisture_value(&mut self) -> u16 {
        self.adc_drv.read(&mut self.adc_pin_moist).unwrap_or(0)
    }
}

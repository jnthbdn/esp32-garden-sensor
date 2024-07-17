use std::time::SystemTime;

use esp_idf_hal::{
    delay::Delay,
    gpio::{Input, InputPin, Output, OutputPin, PinDriver},
};

const HALF_SPEED_SOUND: f32 = 170.0;

pub struct HCSR04Sensor<'a, PEN: OutputPin, PTRIG: OutputPin, PECHO: InputPin> {
    pin_enable: PinDriver<'a, PEN, Output>,
    pin_trigger: PinDriver<'a, PTRIG, Output>,
    pin_echo: PinDriver<'a, PECHO, Input>,

    dist_low: f32,
    dist_high: f32,
}

impl<'a, PEN: OutputPin, PTRIG: OutputPin, PECHO: InputPin> HCSR04Sensor<'a, PEN, PTRIG, PECHO> {
    pub fn new(
        pin_enable: PEN,
        pin_trigger: PTRIG,
        pin_echo: PECHO,
        dist_low: f32,
        dist_high: f32,
    ) -> anyhow::Result<Self> {
        let mut s = Self {
            pin_enable: PinDriver::output(pin_enable)?,
            pin_trigger: PinDriver::output(pin_trigger)?,
            pin_echo: PinDriver::input(pin_echo)?,

            dist_low,
            dist_high,
        };

        s.pin_enable.set_low()?;
        s.pin_trigger.set_low()?;

        Ok(s)
    }

    pub fn read_raw_value(&mut self) -> u128 {
        let delay = Delay::new_default();

        let _ = self.pin_enable.set_high();
        delay.delay_ms(100);

        let _ = self.pin_trigger.set_high();
        delay.delay_us(5);
        let _ = self.pin_trigger.set_low();

        let result = self.measure_echo_pulse();

        let _ = self.pin_enable.set_low();
        delay.delay_ms(100);

        result
    }

    fn measure_echo_pulse(&mut self) -> u128 {
        let start_listen_echo = SystemTime::now();
        let mut start_echo_high: Option<SystemTime> = None;

        loop {
            if start_echo_high.is_none() && self.pin_echo.is_high() {
                start_echo_high = Some(SystemTime::now());
            }

            if start_echo_high.is_some() && self.pin_echo.is_low() {
                return start_echo_high.unwrap().elapsed().unwrap().as_micros();
            }

            if start_listen_echo.elapsed().unwrap().as_millis() > 60 {
                log::info!("Timeout !");
                return 0;
            }
        }
    }

    pub fn get_distance_mm(&mut self) -> f32 {
        let pulse_us = self.read_raw_value() as f32;

        ((pulse_us / 1_000_000.0) * HALF_SPEED_SOUND) * 1_000.0
    }

    pub fn get_level(&mut self) -> f32 {
        let dist_mm = self.get_distance_mm();

        let slope: f32 = 100.0 / (self.dist_high - self.dist_low);
        (slope * (dist_mm - self.dist_low)).clamp(0.0, 100.0)
    }
}

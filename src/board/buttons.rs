use esp_idf_hal::gpio::*;

pub struct Buttons<'a> {
    #[cfg(feature = "moisture-sensor")]
    pub settings: PinDriver<'a, Gpio5, Input>,
}

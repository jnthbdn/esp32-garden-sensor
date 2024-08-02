use esp_idf_svc::hal::gpio::*;

pub struct Buttons<'a> {
    pub settings: PinDriver<'a, Gpio5, Input>,
}

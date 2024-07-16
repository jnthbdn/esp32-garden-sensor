use esp_idf_hal::gpio::*;

pub struct OnBoardLed<'a> {
    pub orange: PinDriver<'a, Gpio6, Output>,
    pub green: PinDriver<'a, Gpio7, Output>,
}

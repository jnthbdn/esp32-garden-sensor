pub struct MoistureSensor {
    v_high: f32,
    v_low: f32,
}

impl MoistureSensor {
    pub fn new(voltage_high: f32, voltage_low: f32) -> Self {
        Self {
            v_high: voltage_high,
            v_low: voltage_low,
        }
    }

    pub fn get_moisture_level(&self, adc_value: f32) -> f32 {
        let slope: f32 = 100.0 / (self.v_high - self.v_low);
        (slope * (adc_value - self.v_low)).clamp(0.0, 100.0)
    }
}

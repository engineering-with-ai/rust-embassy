//! Temperature driver implementations

/// Generic temperature sensor driver (SHTC3 implementation).
#[cfg(feature = "shtcx")]
pub struct TemperatureDriver<I2C> {
    /// SHTC3 sensor instance for I2C communication
    sensor: shtcx::ShtCx<shtcx::sensor_class::Sht2Gen, I2C>,
}

#[cfg(all(feature = "shtcx", feature = "embassy-time"))]
impl<I2C> TemperatureDriver<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    /// Creates a new temperature driver instance.
    ///
    /// # Arguments
    /// * `i2c` - I2C bus interface
    pub fn new(i2c: I2C) -> Self {
        Self {
            sensor: shtcx::shtc3(i2c),
        }
    }

    /// Reads temperature from sensor in Celsius.
    ///
    /// # Returns
    /// Temperature in degrees Celsius
    pub fn read_celsius(&mut self) -> f32 {
        let mut delay = embassy_time::Delay;
        match self
            .sensor
            .measure(shtcx::PowerMode::NormalMode, &mut delay)
        {
            Ok(measurement) => measurement.temperature.as_degrees_celsius(),
            Err(_) => 25.0, // Fallback on error
        }
    }
}

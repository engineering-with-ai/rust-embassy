//! Temperature conversion client utilities

#[cfg(feature = "shtcx")]
use crate::temperature::temperature_driver::TemperatureDriver;

/// High-level temperature client that manages driver internally.
#[cfg(feature = "shtcx")]
pub struct TemperatureClient<I2C> {
    /// Internal temperature sensor driver
    driver: TemperatureDriver<I2C>,
}

#[cfg(feature = "shtcx")]
impl<I2C> TemperatureClient<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    /// Creates a new temperature client with I2C driver.
    ///
    /// # Arguments
    /// * `i2c` - I2C bus interface for temperature sensor
    ///
    /// # Returns
    /// New TemperatureClient instance
    pub fn new(i2c: I2C) -> Self {
        Self {
            driver: TemperatureDriver::new(i2c),
        }
    }

    /// Reads temperature from sensor and returns Fahrenheit.
    ///
    /// # Returns
    /// Temperature in Fahrenheit
    pub fn read_fahrenheit(&mut self) -> f64 {
        let celsius = self.driver.read_celsius();
        celsius_to_fahrenheit(celsius as f64)
    }
}

/// Converts Celsius to Fahrenheit.
///
/// # Arguments
/// * `celsius` - Temperature in Celsius
///
/// # Returns
/// Temperature in Fahrenheit
///
#[inline(never)]
pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

//! Application library for rust-embassy template.
//!
//! This library provides temperature monitoring and publishing.

#![cfg_attr(not(test), no_std)]

#[cfg(all(feature = "shtcx", feature = "rust-mqtt", feature = "embassy-net"))]
use crate::temperature::temperature_client::TemperatureClient;

/// MQTT client for publishing sensor data.
pub mod mqtt;
/// WiFi and network stack setup.
pub mod network;
/// Temperature sensor client.
pub mod temperature;

/// System loop rate in seconds.
pub const SYSTEM_RATE: u64 = 2;

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(feature = "integration-test")]
pub const MODE: &str = "development";

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(not(feature = "integration-test"))]
pub const MODE: &str = "beta";

/// Main application function that reads temperature and publishes to MQTT.
///
/// Initializes client and runs main application loop.
///
/// # Arguments
/// * `i2c` - I2C bus interface for temperature sensor
/// * `stack` - Embassy network stack for MQTT connectivity
///
/// # Returns
/// Ok(()) on successful initialization and first read (used for testing)
#[cfg(all(feature = "shtcx", feature = "rust-mqtt", feature = "embassy-net"))]
pub async fn run<I2C>(i2c: I2C, stack: &'static embassy_net::Stack<'static>) -> Result<(), ()>
where
    I2C: embedded_hal::i2c::I2c,
{
    use crate::mqtt::{MQTT_TOPIC, Mqtt};
    use defmt::info;
    use embassy_time::{Duration, Timer};

    let mut temp_client = TemperatureClient::new(i2c);
    let mut mqtt_client = Mqtt::init(stack).await;

    info!("Starting application loop ({}s interval)...", SYSTEM_RATE);
    let mut loop_count: u32 = 0;
    loop {
        loop_count += 1;
        let temp_f = temp_client.read_fahrenheit();
        info!("Loop #{}: Temperature = {}F", loop_count, temp_f);

        mqtt_client.publish(MQTT_TOPIC, temp_f).await;

        if MODE == "development" {
            return Ok(());
        }

        Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
    }
}

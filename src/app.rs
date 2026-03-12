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
/// OTA firmware update module.
pub mod ota;
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

/// Check for OTA updates and apply if available.
///
/// # Arguments
/// * `stack` - Embassy network stack for HTTP connectivity
#[cfg(all(feature = "reqwless", target_arch = "riscv32"))]
pub async fn check_and_apply_ota(stack: &'static embassy_net::Stack<'static>) {
    use defmt::info;

    info!("Checking for OTA updates...");
    let mut ota_client = crate::ota::OtaHttpClient::new(stack);
    match ota_client.check_for_update().await {
        Ok(crate::ota::OtaCheckResult::UpToDate) => info!("Firmware is up to date"),
        Ok(crate::ota::OtaCheckResult::UpdateAvailable { .. }) => {
            info!("Update available, fetching firmware info...");

            match ota_client.get_firmware_info().await {
                Ok(fw_info) => {
                    info!(
                        "Firmware: size={}, checksum={:#x}",
                        fw_info.size, fw_info.checksum
                    );

                    #[cfg(feature = "esp-hal-ota")]
                    {
                        match crate::ota::OtaWriter::new() {
                            Ok(mut writer) => {
                                if writer.begin(fw_info.size, fw_info.checksum).is_ok() {
                                    info!("OTA initialized, downloading...");
                                    let result = ota_client
                                        .download_firmware(|_offset, data| {
                                            let _ = writer.write_chunk(data);
                                        })
                                        .await;

                                    match result {
                                        Ok(_) => {
                                            info!("Download complete, finalizing...");
                                            if writer.finalize(true, true).is_ok() {
                                                info!("OTA success! Rebooting...");
                                                crate::ota::OtaWriter::reboot();
                                            }
                                        }
                                        Err(_) => info!("Download failed"),
                                    }
                                }
                            }
                            Err(_) => info!("Failed to create OtaWriter"),
                        }
                    }

                    #[cfg(not(feature = "esp-hal-ota"))]
                    {
                        let result = ota_client
                            .download_firmware(|offset, data| {
                                info!("Chunk: offset={}, size={}", offset, data.len());
                            })
                            .await;
                        match result {
                            Ok(crc) => info!("Download complete, CRC={:#x}", crc),
                            Err(_) => info!("Download failed"),
                        }
                    }
                }
                Err(_) => info!("Failed to get firmware info"),
            }
        }
        Err(_) => info!("OTA check failed (continuing anyway)"),
    }
}

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

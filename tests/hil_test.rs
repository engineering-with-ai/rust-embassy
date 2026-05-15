//! Hardware-in-Loop tests for ESP32 firmware.
//!
//! Tests include:
//! - MQTT integration: temperature sensor publishes to MQTT broker

mod fixtures;

use fixtures::containers::start_mqtt_broker;
use rumqttc::v5::{
    AsyncClient, Event, MqttOptions,
    mqttbytes::{QoS, v5::Packet},
};
use std::error::Error;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Topic where temperature readings are published.
const MQTT_TOPIC: &str = "test/temp/F";

/// Maximum time to wait for MQTT message (in seconds).
const MQTT_TIMEOUT_SECS: u64 = 80;

/// Flash firmware and reset device (detached — no probe-rs process stays running).
///
/// Reason: `probe-rs run` holds the debug interface during boot, which disrupts
/// WiFi WPA handshake timing on macOS. `probe-rs attach` resets ESP32-C3 via
/// shared USB-JTAG. Since no HIL test parses RTT output (they verify via broker
/// or server state), we flash + reset and let the device run headless.
fn flash_and_reset(elf: &str) -> Result<(), Box<dyn Error>> {
    let download = Command::new("probe-rs")
        .args(["download", "--chip=esp32c3", elf])
        .status()?;
    if !download.success() {
        return Err("probe-rs download failed".into());
    }

    let reset = Command::new("probe-rs")
        .args(["reset", "--chip=esp32c3"])
        .status()?;
    if !reset.success() {
        return Err("probe-rs reset failed".into());
    }

    Ok(())
}

#[tokio::test]
async fn test_hil_mqtt_integration() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    // Arrange - Start MQTT broker
    println!(
        "[{:.1}s] 🏗️  ARRANGE: Starting MQTT broker...",
        start.elapsed().as_secs_f32()
    );
    let broker = start_mqtt_broker().await?;
    println!(
        "[{:.1}s] ✅ HiveMQ broker started on port {}",
        start.elapsed().as_secs_f32(),
        broker.port
    );

    // Check required environment variables
    println!(
        "[{:.1}s] 🔍 Checking environment variables...",
        start.elapsed().as_secs_f32()
    );
    match std::env::var("WIFI_PASSWORD") {
        Ok(pwd) => println!(
            "[{:.1}s] ✅ WIFI_PASSWORD is set (length: {})",
            start.elapsed().as_secs_f32(),
            pwd.len()
        ),
        Err(_) => {
            println!(
                "[{:.1}s] ❌ ERROR: WIFI_PASSWORD environment variable not set!",
                start.elapsed().as_secs_f32()
            );
            return Err("WIFI_PASSWORD environment variable must be set".into());
        }
    }

    // Build firmware with dynamic MQTT port from testcontainer
    println!(
        "[{:.1}s] 🏗️  ACT: Building firmware with MQTT_PORT={}...",
        start.elapsed().as_secs_f32(),
        broker.port
    );
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin=rs")
        .arg("--features=defmt,esp-hal,embassy-time,esp-hal-embassy,rtt-target,esp-alloc,embassy-executor,embassy-net,esp-bootloader-esp-idf,critical-section,esp-wifi,smoltcp,static_cell,rust-mqtt,heapless,shtcx,embedded-hal")
        .env("MQTT_PORT", broker.port.to_string())
        .status()?;

    if !build_status.success() {
        println!(
            "[{:.1}s] ❌ ERROR: Firmware build failed!",
            start.elapsed().as_secs_f32()
        );
        return Err("Firmware build failed".into());
    }
    println!(
        "[{:.1}s] ✅ Firmware built successfully",
        start.elapsed().as_secs_f32()
    );

    // Subscribe to temperature topic BEFORE flashing so we don't miss the first message
    println!(
        "[{:.1}s] 📡 Setting up MQTT test client...",
        start.elapsed().as_secs_f32()
    );
    let mqttoptions = MqttOptions::new("hil_test_client", "localhost", broker.port);
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    println!(
        "[{:.1}s] 📝 Subscribing to topic: {}",
        start.elapsed().as_secs_f32(),
        MQTT_TOPIC
    );
    client.subscribe(MQTT_TOPIC, QoS::AtLeastOnce).await?;
    println!(
        "[{:.1}s] ✅ Subscribed to MQTT topic successfully",
        start.elapsed().as_secs_f32()
    );

    // Flash firmware and reset device (runs headless — no probe-rs attach)
    println!(
        "[{:.1}s] 📱 Flashing firmware...",
        start.elapsed().as_secs_f32()
    );
    flash_and_reset("target/riscv32imc-unknown-none-elf/release/rs")?;
    println!(
        "[{:.1}s] ✅ Firmware flashed and device reset",
        start.elapsed().as_secs_f32()
    );

    // Act - Wait for temperature message (firmware runs automatically after flash)
    println!(
        "[{:.1}s] 🎯 ASSERT: Waiting for MQTT messages (timeout: {}s)...",
        start.elapsed().as_secs_f32(),
        MQTT_TIMEOUT_SECS
    );

    // Assert - Verify temperature message received within timeout
    let result = timeout(Duration::from_secs(MQTT_TIMEOUT_SECS), async {
        loop {
            let notification = eventloop.poll().await?;
            println!(
                "[{:.1}s] 📨 Received MQTT event: {:?}",
                start.elapsed().as_secs_f32(),
                std::mem::discriminant(&notification)
            );

            if let Event::Incoming(Packet::Publish(p)) = notification {
                let payload = String::from_utf8(p.payload.to_vec())
                    .map_err(|_| "Invalid UTF-8 in payload")?;

                println!(
                    "[{:.1}s] 📄 Received payload: {}",
                    start.elapsed().as_secs_f32(),
                    payload
                );

                let temp_f: f32 = payload
                    .parse()
                    .map_err(|_| "Payload is not a valid float")?;

                // Validate temperature is in reasonable range (50-104°F)
                assert!(
                    (50.0..=104.0).contains(&temp_f),
                    "Temperature {} is outside reasonable range",
                    temp_f
                );

                println!(
                    "[{:.1}s] ✅ Test passed! Received valid temperature: {}°F",
                    start.elapsed().as_secs_f32(),
                    temp_f
                );
                return Ok::<(), Box<dyn Error + Send + Sync>>(());
            }
        }
    })
    .await;

    // Handle timeout or success
    match result {
        Ok(_) => {
            println!(
                "[{:.1}s] 🎉 SUCCESS: Test completed within timeout",
                start.elapsed().as_secs_f32()
            );
        }
        Err(_) => {
            println!(
                "[{:.1}s] ⏰ TIMEOUT: No MQTT message received within {} seconds",
                start.elapsed().as_secs_f32(),
                MQTT_TIMEOUT_SECS
            );
            println!(
                "[{:.1}s] 🔍 This could indicate:",
                start.elapsed().as_secs_f32()
            );
            println!(
                "[{:.1}s]    - WiFi connection issues on ESP32",
                start.elapsed().as_secs_f32()
            );
            println!(
                "[{:.1}s]    - MQTT broker connection issues",
                start.elapsed().as_secs_f32()
            );
            println!(
                "[{:.1}s]    - Temperature sensor reading issues",
                start.elapsed().as_secs_f32()
            );
            println!(
                "[{:.1}s]    - Firmware flash/run issues",
                start.elapsed().as_secs_f32()
            );

            return Err(format!("Test timed out after {} seconds", MQTT_TIMEOUT_SECS).into());
        }
    }

    Ok(())
}

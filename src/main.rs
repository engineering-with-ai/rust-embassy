//! ESP32-C3 Embassy firmware application.
//!
//! Main binary for ESP32-C3 with Embassy async runtime, WiFi, and MQTT support.

#![no_std]
#![no_main]
#![allow(missing_docs)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;

/// System loop rate in seconds.
const SYSTEM_RATE: u64 = 2;

/// Panic handler for embedded no_std environment.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

/// Main firmware entry point.
#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    info!("=== ESP32-C3 Firmware Starting ===");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    info!("Connecting to WiFi...");
    // Initialize WiFi and network stack (also initializes embassy)
    let stack = app::network::setup_network(&spawner, peripherals).await;
    info!("WiFi connected!");

    // Retrieve peripherals for I2C (setup_network doesn't use these)
    let peripherals = unsafe { esp_hal::peripherals::Peripherals::steal() };
    let i2c =
        esp_hal::i2c::master::I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
            .unwrap()
            .with_sda(peripherals.GPIO10)
            .with_scl(peripherals.GPIO8);

    info!("Initializing MQTT client...");
    let mut mqtt = app::mqtt::Mqtt::init(stack).await;
    info!("MQTT client ready!");

    // Initialize temperature sensor
    let mut temp_client = app::temperature::temperature_client::TemperatureClient::new(i2c);

    info!("Starting application loop ({}s interval)...", SYSTEM_RATE);
    let mut loop_count = 0;
    loop {
        loop_count += 1;

        let temp_f = temp_client.read_fahrenheit();
        info!("Loop #{}: Temperature = {}F", loop_count, temp_f);

        // Publish temperature to MQTT
        mqtt.publish(app::mqtt::MQTT_TOPIC, temp_f).await;

        Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
    }
}

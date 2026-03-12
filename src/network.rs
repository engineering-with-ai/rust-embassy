//! WiFi and network stack setup for ESP32-C3.
//!
//! Handles WiFi connection, network stack initialization, and connectivity management.

#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use defmt::info;
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use embassy_executor::Spawner;
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use embassy_net::{Runner, Stack, StackResources};
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use embassy_time::{Duration, Timer};
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use esp_hal::{peripherals::Peripherals, rng::Rng, timer::timg::TimerGroup};
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use esp_wifi::{
    EspWifiController,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
};
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
use static_cell::StaticCell;

/// WiFi SSID baked at compile time.
///
/// Set via WIFI_SSID in cfg.yml based on ENV environment variable.
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
const WIFI_SSID: &str = env!("WIFI_SSID");

/// WiFi password baked at compile time.
///
/// Set via WIFI_PASSWORD environment variable during build.
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

/// Static cell macro for creating 'static references.
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: StaticCell<$t> = StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
}

/// WiFi connection task - handles connect/reconnect logic.
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("🚀 Starting WiFi connection task");
    info!("📡 Device capabilities: {:?}", controller.capabilities());
    info!("🔐 Target SSID: {}", WIFI_SSID);
    info!("🔑 Password length: {} chars", WIFI_PASSWORD.len());

    loop {
        let current_state = esp_wifi::wifi::wifi_state();
        info!("🔍 Current WiFi state: {:?}", current_state);

        if current_state == WifiState::StaConnected {
            info!("✅ WiFi already connected, waiting for disconnect event...");
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            info!("🔌 WiFi disconnected, waiting 5s before retry...");
            Timer::after(Duration::from_millis(5000)).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            info!("🔧 WiFi not started, configuring and starting...");
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: WIFI_SSID.into(),
                password: WIFI_PASSWORD.into(),
                ..Default::default()
            });

            info!("📝 Setting WiFi configuration...");
            controller.set_configuration(&client_config).unwrap();

            info!("⚡ Starting WiFi driver...");
            controller.start_async().await.unwrap();
            info!("✅ WiFi driver started successfully!");
        }

        info!(
            "🔗 Attempting to connect to WiFi network '{}'...",
            WIFI_SSID
        );
        match controller.connect_async().await {
            Ok(_) => {
                info!("✅ WiFi connected successfully!");
                info!("📊 Final WiFi state: {:?}", esp_wifi::wifi::wifi_state());
            }
            Err(e) => {
                info!("❌ Failed to connect to WiFi: {:?}", e);
                info!("⏳ Waiting 5 seconds before retry...");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

/// Network stack runner task.
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

/// Initialize WiFi and network stack.
///
/// Sets up WiFi controller, creates embassy-net stack with DHCP,
/// spawns connection tasks, and waits for network connectivity.
///
/// # Arguments
/// * `spawner` - Embassy executor spawner for background tasks
/// * `peripherals` - ESP32-C3 peripherals (WIFI, TIMG0, RNG)
///
/// # Returns
/// Static reference to configured network stack ready for use
#[cfg(all(feature = "embassy-net", feature = "esp-wifi"))]
pub async fn setup_network(spawner: &Spawner, peripherals: Peripherals) -> &'static Stack<'static> {
    // Initialize embassy executor
    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("Embassy initialized!");

    let mut rng = Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);

    let esp_wifi_ctrl = mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(timer1.timer0, rng).expect("Failed to initialize WIFI/BLE controller")
    );

    // Create WiFi controller and interfaces
    let (controller, interfaces) = esp_wifi::wifi::new(esp_wifi_ctrl, peripherals.WIFI)
        .expect("Failed to initialize WIFI controller");

    let wifi_interface = interfaces.sta;

    // Configure network stack with DHCP
    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Initialize network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    // Make stack 'static for MQTT client
    let stack = mk_static!(Stack<'static>, stack);

    // Spawn WiFi connection and network tasks
    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    // Wait for link up
    info!("⏳ Waiting for WiFi link to come up...");
    let mut link_wait_count = 0;
    loop {
        if stack.is_link_up() {
            break;
        }
        link_wait_count += 1;
        if link_wait_count % 10 == 0 {
            info!(
                "🕐 Still waiting for WiFi link... ({}s elapsed)",
                link_wait_count / 2
            );
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    info!("✅ WiFi link is up!");

    // Wait for IP address
    info!("⏳ Waiting for DHCP to assign IP address...");
    let mut ip_wait_count = 0;
    loop {
        if let Some(config) = stack.config_v4() {
            info!("✅ Got IP address: {}", config.address);
            info!("🌐 Gateway: {:?}", config.gateway);
            info!("🔍 DNS servers: {:?}", config.dns_servers);
            break;
        }
        ip_wait_count += 1;
        if ip_wait_count % 10 == 0 {
            info!(
                "🕐 Still waiting for DHCP IP... ({}s elapsed)",
                ip_wait_count / 2
            );
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    stack
}

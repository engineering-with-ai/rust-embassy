//! Build script for ESP32-C3 Embassy firmware.
//!
//! Configures linker scripts, provides helpful error messages for common build issues,
//! and bakes environment variables into firmware at compile time.

use serde::Deserialize;
use std::collections::HashMap;

/// Configuration structure matching cfg.yml format.
#[derive(Debug, Deserialize)]
struct Config {
    /// WiFi network SSID to connect to
    wifi_ssid: String,
    /// MQTT broker hostname or IP address
    mqtt_host: String,
}

fn main() {
    // Rerun if cfg.yml changes
    println!("cargo:rerun-if-changed=cfg.yml");
    println!("cargo:rerun-if-env-changed=ENV");
    println!("cargo:rerun-if-env-changed=WIFI_PASSWORD");
    println!("cargo:rerun-if-env-changed=MQTT_PORT");
    println!("cargo:rerun-if-env-changed=FIRMWARE_VERSION");

    // Determine environment (default to "local")
    let env = std::env::var("ENV").unwrap_or_else(|_| "local".to_string());

    // Load and parse cfg.yml
    let cfg_content = std::fs::read_to_string("cfg.yml").expect("Failed to read cfg.yml");
    let all_configs: HashMap<String, Config> =
        serde_yaml::from_str(&cfg_content).expect("Failed to parse cfg.yml");
    let config = all_configs
        .get(&env)
        .unwrap_or_else(|| panic!("Environment '{}' not found in cfg.yml", env));

    // Get WiFi SSID from config
    let wifi_ssid = &config.wifi_ssid;

    // Get WiFi password from environment variable (required)
    let wifi_password =
        std::env::var("WIFI_PASSWORD").expect("WIFI_PASSWORD environment variable must be set");

    // Get MQTT host from config
    let mqtt_host = &config.mqtt_host;

    // Get MQTT port from env var (for HIL tests) or default to 1883
    let mqtt_port = std::env::var("MQTT_PORT").unwrap_or_else(|_| "1883".to_string());

    // Get firmware version from env var or use Cargo.toml version
    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let firmware_version =
        std::env::var("FIRMWARE_VERSION").unwrap_or_else(|_| pkg_version.clone());

    // Bake values into firmware
    println!("cargo:rustc-env=WIFI_SSID={}", wifi_ssid);
    println!("cargo:rustc-env=WIFI_PASSWORD={}", wifi_password);
    println!("cargo:rustc-env=MQTT_HOST={}", mqtt_host);
    println!("cargo:rustc-env=MQTT_PORT={}", mqtt_port);
    println!("cargo:rustc-env=FIRMWARE_VERSION={}", firmware_version);

    // Skip embedded linker configuration for host unit tests
    let target = std::env::var("TARGET").unwrap();
    if target.contains("riscv32") {
        linker_be_nice();
        println!("cargo:rustc-link-arg-tests=-Tembedded-test.x");
        println!("cargo:rustc-link-arg=-Tdefmt.x");
        // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
        println!("cargo:rustc-link-arg=-Tlinkall.x");
    }
}

/// Provides helpful error messages for common linker errors.
///
/// Acts as an error handling script for the linker, catching common issues
/// and providing actionable suggestions.
fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_defmt_timestamp" => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                "esp_wifi_preempt_enable"
                | "esp_wifi_preempt_yield_task"
                | "esp_wifi_preempt_task_create" => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-wifi` has no scheduler enabled. Make sure you have the `builtin-scheduler` feature enabled, or that you provide an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}

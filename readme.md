# rust-embassy 📟

![](https://img.shields.io/gitlab/pipeline-status/engineering-with-ai/rust-embassy?branch=main&logo=gitlab)
![](https://gitlab.com/engineering-with-ai/rust-embassy/badges/main/coverage.svg)
![](https://img.shields.io/badge/1.93.0-gray?logo=rust)
![](https://img.shields.io/badge/4.1.0-gray?logo=espressif)
![](https://img.shields.io/badge/embassy-0.6.0-orange)
![](https://img.shields.io/badge/mqtt-gray?logo=mqtt)

> Embedded Rust template for ESP32-C3 using Embassy async runtime, WiFi + MQTT, and hardware-in-loop testing with probe-rs.

## Pre-requisites

```shell
cargo install cargo-cmd cargo-audit cargo-udeps --locked
cargo install espflash cargo-binstall cargo-tarpaulin cargo2junit
cargo binstall probe-rs-tools

# Replace <target> with aarch64-apple-darwin or x86_64-unknown-linux-gnu
rustup component add rust-src --toolchain nightly-<target>
```

## Verify probe-rs connection

```bash
probe-rs list                          # list connected probes
probe-rs info                          # chip info
probe-rs chip list | grep -i esp32c3   # confirm target support
probe-rs reset --chip esp32c3          # reset chip
```

Expected: **ESP JTAG** probe, **RISC-V ESP32-C3** chip, **JTAG** protocol (not SWD), manufacturer **Espressif Systems** (1554).

## Device paths

| OS | Path |
|---|---|
| macOS | `/dev/tty.usbmodem*` |
| Linux | `/dev/ttyACM*` or `/dev/ttyUSB*` |

## Network setup

When switching networks:

1. Update `cfg.yml`:
   ```yaml
   wifi_ssid: "your-network"
   mqtt_host: "192.168.1.XXX"   # host machine IP
   ```

2. Export WiFi password (never commit):
   ```bash
   export WIFI_PASSWORD="your-password"
   ```

3. Run hardware-in-loop to verify:
   ```bash
   cargo cmd hardware-in-loop
   ```

Find host IP: `ifconfig | grep "inet " | grep -v 127.0.0.1`

## Debugging stack overflow

**Symptoms:** USB disconnects immediately after flash, device crashes in WiFi stack (`g_wpa_supp`), no useful error messages.

**Root cause:** Large stack allocations. ESP32-C3 has ~8KB stack by default.

```rust
// ❌ 6KB on stack
static mut TCP_BUFS: TcpClientState<1, 6144, 1024> = TcpClientState::new();
let mut rx_buf = [0u8; 6144];

// ✅ 1KB on stack
static mut TCP_BUFS: TcpClientState<1, 1024, 256> = TcpClientState::new();
let mut rx_buf = [0u8; 1024];
```

**Strategy:**
1. Isolate — remove features (OTA, MQTT) to find which causes the crash
2. Baseline — test minimal firmware to confirm hardware works
3. Binary search — narrow to a specific function
4. Inspect stack — look for large arrays, buffers, or deep recursion
5. Power cycle — if device is corrupted:
   ```bash
   espflash erase-flash
   # unplug/replug USB
   ```

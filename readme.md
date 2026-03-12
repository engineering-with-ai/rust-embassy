# [rust-embassy] 📟
![](https://img.shields.io/gitlab/pipeline-status/engineering-with-ai/software-rust-embassy?branch=main&logo=gitlab)
![](https://gitlab.com/engineering-with-ai/software-rust-embassy/badges/main/coverage.svg)
![](https://img.shields.io/badge/1.90-gray?logo=rust)
![](https://img.shields.io/badge/4.1.0-gray?logo=espressif)
![](https://img.shields.io/badge/embassy-0.6.0-orange)
![](https://img.shields.io/badge/drogue-blue)
![](https://img.shields.io/badge/mqtt-gray?logo=mqtt)

## Pre-Requisites

```shell
cargo install cargo-cmd \
              cargo-audit \
              cargo-udeps --locked \
              espflash--locked \
              cargo-bininstall \
              cargo-tarpaulin \
              cargo2junit  && \

cargo binistall probe-rs-tools
rustup component add rust-src --toolchain nightly-<aarch64-apple-darwin|x86_64-unknown-linux-gnu>
```

## Verify probe-rs Connection

### List probes
```bash
probe-rs list
```

### Get chip info
```bash
probe-rs info
```

### Check target support
```bash
probe-rs chip list | grep -i esp32c3
```

### Reset chip
```bash
probe-rs reset --chip esp32c3
```

## Device Paths

**macOS**: `/dev/tty.usbmodem*`
```bash
ls -la /dev/tty.usbmodem*
```

**Linux**: `/dev/ttyACM*` or `/dev/ttyUSB*`
```bash
ls -la /dev/ttyACM* /dev/ttyUSB*
```

## Expected Output

- **Probe**: ESP JTAG (EspJtag)
- **Chip**: RISC-V ESP32-C3
- **Protocol**: JTAG (not SWD)
- **Manufacturer**: Espressif Systems (1554)

## Network Setup

When switching networks, update:

1. **cfg.yml**:
   ```yaml
   wifi_ssid: "your-network"
   mqtt_host: "192.168.1.XXX"  # Your machine's IP
   ```

2. **Environment**:
   ```bash
   export WIFI_PASSWORD="your-password"
   ```

3. **Test**:
   ```bash
   WIFI_PASSWORD=your-password cargo cmd hardware-in-loop
   ```

Find your IP: `ifconfig | grep "inet " | grep -v 127.0.0.1`

## OTA Updates

### Protocol Extension

The drogue-ajour protocol only provides checksum in the final `Swap` command. For streaming writes to flash (without RAM buffering), we extended the server with `/dfu/info` endpoint that returns size and checksum upfront:

```json
{"size":102400,"checksum":3135550226,"version":"2.0.0"}
```

### Flashing with OTA Support

OTA requires partition table. Use hybrid workflow:

```bash
# 1. Flash with partition table (espflash)
espflash flash --chip esp32c3 --partition-table partitions.csv target/riscv32imc-unknown-none-elf/release/rs

# 2. Attach for RTT output (probe-rs)
probe-rs attach --chip=esp32c3 target/riscv32imc-unknown-none-elf/release/rs
```

**Note**: `probe-rs run` alone won't work for OTA - esp-hal-ota requires 2+ OTA partitions which are only present when flashed with the partition table.

## Debugging: Stack Overflow

### Symptoms
- USB disconnects immediately after flash
- Device crashes in WiFi stack (g_wpa_supp)
- No useful error messages

### Root Cause
Large stack allocations in embedded code. ESP32-C3 has ~8KB stack by default.

**Bad** (caused stack overflow):
```rust
static mut TCP_BUFS: TcpClientState<1, 6144, 1024> = TcpClientState::new();
let mut rx_buf = [0u8; 6144];  // 6KB on stack!
```

**Fixed**:
```rust
static mut TCP_BUFS: TcpClientState<1, 1024, 256> = TcpClientState::new();
let mut rx_buf = [0u8; 1024];  // 1KB - safe
```

### Debugging Strategy
1. **Isolate**: Remove features (OTA, MQTT) to find which causes crash
2. **Baseline**: Test minimal firmware to confirm hardware works
3. **Binary search**: Narrow down to specific function
4. **Inspect stack**: Look for large arrays, buffers, or deep recursion
5. **Power cycle**: If device is corrupted, full chip erase + power cycle:
   ```bash
   espflash erase-flash
   # Unplug/replug USB
   ```

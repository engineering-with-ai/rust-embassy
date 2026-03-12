# Resume Context: Integration Test Issue

## Current Status
Integration test (`cargo cmd integration`) failing with:
```
Error: The target did not respond with test list until timeout.
```

## What We Tried
1. **embedded-test 0.6.0** (original): "test list timeout" error
2. **embedded-test 0.7.0**: Got past timeout, but stack overflow in RTT init
3. **defmt-rtt instead of rtt-target**: Same stack overflow
4. **Chip erase + power cycle**: No change
5. **Reverted to pre-OTA code**: Same error - NOT a code regression

## Key Findings
- Main firmware runs fine with probe-rs (WiFi, MQTT, OTA all work)
- embedded-test 0.7.0 actually runs tests but crashes with "Stack pointer is too low to unwind" in RTT buffer init
- ESP32-C3 has ~8KB stack, RTT buffers use significant stack space
- Issue is specific to embedded-test harness, not probe-rs or device

## Relevant Issues
- https://github.com/probe-rs/embedded-test/issues/37 (ESP32-S3 similar issues)
- https://github.com/probe-rs/probe-rs/issues/2354 (ESP32 test timeout)

## Environment
- probe-rs 0.30.0
- embedded-test 0.6.0 (in Cargo.toml)
- ESP32-C3 via JTAG

## Files Modified During Session
- `src/ota/http_client.rs` - Added get_firmware_info(), JSON parsing
- `src/ota/mod.rs` - Export FirmwareInfo
- `src/main.rs` - OTA flow with info fetch
- `tests/fixtures/ota_server.rs` - Added /dfu/info endpoint
- `tests/hil_test.rs` - 4 OTA tests (all pass on host)
- `readme.md` - Added OTA and debugging docs
- `Cargo.toml` - OTA deps restored

## OTA Work Status
OTA implementation is COMPLETE and working:
- `/dfu/info` endpoint for upfront size+checksum
- `download_firmware()` with streaming chunks
- HIL tests pass: check, update_available, download, flash_write

## Next Steps to Try
1. Check if integration test ever worked (CI logs?)
2. Try older probe-rs version
3. Try reducing RTT buffer size
4. Try semihosting instead of RTT for test output
5. Power cycle device and retry

## Commands
```bash
# Run integration test
cargo cmd integration

# Run HIL tests (these work)
cargo cmd hardware-in-loop

# Run main firmware (this works)
probe-rs run --chip esp32c3 target/riscv32imc-unknown-none-elf/release/rs
```

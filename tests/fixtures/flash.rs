//! ESP32 flashing helpers using probe-rs for RTT debugging.

use std::process::{Child, Command, Stdio};

/// Path to the release binary.
const RELEASE_BIN: &str = "target/riscv32imc-unknown-none-elf/release/rs";

/// Attaches to running firmware via probe-rs for RTT debugging.
///
/// Does NOT reflash - just connects to the running device.
///
/// # Arguments
/// * `elf_path` - Optional custom ELF path (defaults to release binary)
///
/// # Returns
/// Child process handle for the probe-rs attach process.
pub fn attach_for_rtt(elf_path: Option<&str>) -> Result<Child, String> {
    let path = elf_path.unwrap_or(RELEASE_BIN);

    Command::new("probe-rs")
        .args(["attach", "--chip=esp32c3", path])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn probe-rs attach: {}", e))
}

/// Resets the device without reflashing.
pub fn reset_device() -> Result<(), String> {
    let status = Command::new("probe-rs")
        .args(["reset", "--chip=esp32c3"])
        .status()
        .map_err(|e| format!("Failed to run probe-rs reset: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("probe-rs reset failed".into())
    }
}

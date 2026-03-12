//! ESP32 flashing helpers using the hybrid workflow.
//!
//! Uses espflash for partition-aware flashing and probe-rs for RTT debugging.

use std::process::{Child, Command, Stdio};

/// Path to the release binary.
const RELEASE_BIN: &str = "target/riscv32imc-unknown-none-elf/release/rs";

/// Path to the partition table.
const PARTITION_TABLE: &str = "partitions.csv";

/// Flashes firmware using espflash with partition table support.
///
/// # Arguments
/// * `elf_path` - Optional custom ELF path (defaults to release binary)
///
/// # Returns
/// Result indicating success or failure.
pub fn flash_with_partitions(elf_path: Option<&str>) -> Result<(), String> {
    let path = elf_path.unwrap_or(RELEASE_BIN);

    let status = Command::new("espflash")
        .args([
            "flash",
            "--chip",
            "esp32c3",
            "--partition-table",
            PARTITION_TABLE,
            path,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run espflash: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("espflash flash failed".into())
    }
}

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

/// Creates an OTA-compatible binary from an ELF file.
///
/// # Arguments
/// * `elf_path` - Path to the ELF file
/// * `output_path` - Path for the output binary
///
/// # Returns
/// Result indicating success or failure.
pub fn create_ota_binary(elf_path: &str, output_path: &str) -> Result<(), String> {
    let status = Command::new("espflash")
        .args(["save-image", "--chip", "esp32c3", elf_path, output_path])
        .status()
        .map_err(|e| format!("Failed to run espflash save-image: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("espflash save-image failed".into())
    }
}

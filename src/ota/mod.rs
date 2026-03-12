//! OTA (Over-The-Air) firmware update module.
//!
//! Provides async OTA update functionality using Embassy runtime.
//! Transport-agnostic design allows firmware delivery via HTTP, MQTT, or custom protocols.

#[cfg(all(feature = "reqwless", target_arch = "riscv32"))]
mod http_client;
#[cfg(all(
    feature = "esp-hal-ota",
    feature = "esp-storage",
    target_arch = "riscv32"
))]
mod ota_writer;
#[cfg(test)]
mod protocol_test;

#[cfg(all(feature = "reqwless", target_arch = "riscv32"))]
pub use http_client::{FirmwareInfo, OtaCheckResult, OtaHttpClient};
#[cfg(all(
    feature = "esp-hal-ota",
    feature = "esp-storage",
    target_arch = "riscv32"
))]
pub use ota_writer::OtaWriter;

/// OTA update error types.
#[derive(Debug)]
pub enum OtaError {
    /// Error during OTA begin operation
    BeginFailed,
    /// Error writing firmware data
    WriteFailed,
    /// Error finalizing OTA update
    FinalizeFailed,
    /// Error setting boot partition
    SetBootPartitionFailed,
    /// Network/transport error
    TransportError,
    /// Invalid firmware data
    InvalidFirmware,
}

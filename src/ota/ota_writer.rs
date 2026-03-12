//! OTA firmware writer using esp-hal-ota.
//!
//! Handles writing downloaded firmware chunks to OTA partition and triggering reboot.

use defmt::{error, info};
use esp_hal_ota::Ota;
use esp_storage::FlashStorage;

use super::OtaError;

/// OTA firmware writer for flash operations.
pub struct OtaWriter {
    /// esp-hal-ota instance
    ota: Ota<FlashStorage>,
    /// Whether OTA has been started
    started: bool,
}

impl OtaWriter {
    /// Creates new OTA writer.
    pub fn new() -> Result<Self, OtaError> {
        let flash = FlashStorage::new();
        let ota = Ota::new(flash).map_err(|_| {
            error!("Failed to initialize OTA");
            OtaError::BeginFailed
        })?;

        Ok(Self {
            ota,
            started: false,
        })
    }

    /// Begins OTA update with expected firmware size and CRC32 checksum.
    ///
    /// # Arguments
    /// * `size` - Total firmware size in bytes
    /// * `crc32` - Expected CRC32 checksum for verification
    pub fn begin(&mut self, size: u32, crc32: u32) -> Result<(), OtaError> {
        info!("Starting OTA: size={}, crc32={:#x}", size, crc32);
        self.ota.ota_begin(size, crc32).map_err(|_| {
            error!("Failed to begin OTA");
            OtaError::BeginFailed
        })?;
        self.started = true;
        Ok(())
    }

    /// Writes a firmware chunk to flash.
    ///
    /// # Arguments
    /// * `data` - Chunk data to write
    ///
    /// # Returns
    /// `Ok(true)` if this was the final chunk, `Ok(false)` if more chunks expected
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<bool, OtaError> {
        if !self.started {
            error!("OTA not started");
            return Err(OtaError::WriteFailed);
        }

        let progress = (self.ota.get_ota_progress() * 100.0) as u8;
        info!(
            "Writing chunk: {} bytes, progress: {}%",
            data.len(),
            progress
        );

        match self.ota.ota_write_chunk(data) {
            Ok(complete) => Ok(complete),
            Err(_) => {
                error!("Failed to write OTA chunk");
                Err(OtaError::WriteFailed)
            }
        }
    }

    /// Finalizes OTA update, verifies CRC, and prepares for reboot.
    ///
    /// # Arguments
    /// * `verify_crc` - Whether to verify CRC32 checksum
    /// * `enable_rollback` - Whether to enable rollback on boot failure
    pub fn finalize(&mut self, verify_crc: bool, enable_rollback: bool) -> Result<(), OtaError> {
        info!(
            "Finalizing OTA: verify_crc={}, enable_rollback={}",
            verify_crc, enable_rollback
        );

        self.ota
            .ota_flush(verify_crc, enable_rollback)
            .map_err(|_| {
                error!("OTA finalize failed");
                OtaError::FinalizeFailed
            })?;

        info!("OTA finalized successfully, ready for reboot");
        Ok(())
    }

    /// Gets current OTA progress as percentage (0-100).
    pub fn progress(&self) -> u8 {
        (self.ota.get_ota_progress() * 100.0) as u8
    }

    /// Triggers software reset to boot new firmware.
    pub fn reboot() -> ! {
        info!("Rebooting to new firmware...");
        esp_hal::system::software_reset();
    }
}

//! HTTP client for OTA firmware updates.
//!
//! Provides async HTTP functionality for checking and downloading firmware.

extern crate alloc;

use alloc::vec::Vec;

use super::OtaError;
use defmt::{error, info};
use drogue_ajour_protocol::{Command, Status};
use embassy_net::{
    Stack,
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
};
use reqwless::{
    client::HttpClient,
    request::{Method, RequestBuilder},
};

/// Firmware version baked in at compile time.
const FIRMWARE_VERSION: &[u8] = env!("FIRMWARE_VERSION").as_bytes();

/// OTA server host from config.
const OTA_HOST: &str = env!("OTA_HOST");

/// OTA server port from config.
const OTA_PORT: &str = env!("OTA_PORT");

/// Maximum chunk size for firmware data (4KB).
const MAX_CHUNK_SIZE: usize = 4096;

/// HTTP client for OTA operations.
///
/// Handles firmware update checks and downloads.
pub struct OtaHttpClient<'a> {
    /// Embassy network stack reference
    stack: &'a Stack<'a>,
}

/// Result of an OTA check operation.
#[derive(Debug)]
pub enum OtaCheckResult {
    /// No update available, device is up to date.
    UpToDate,
    /// Update available, contains firmware size and checksum.
    UpdateAvailable {
        /// Total firmware size in bytes.
        size: u32,
        /// Expected CRC32 checksum.
        checksum: u32,
    },
}

/// Firmware metadata from /dfu/info endpoint.
#[derive(Debug)]
pub struct FirmwareInfo {
    /// Firmware size in bytes.
    pub size: u32,
    /// CRC32 checksum.
    pub checksum: u32,
    /// Target version string.
    pub version: Vec<u8>,
}

impl<'a> OtaHttpClient<'a> {
    /// Creates new OTA HTTP client.
    ///
    /// # Arguments
    /// * `stack` - Embassy network stack for HTTP connectivity
    pub fn new(stack: &'a Stack<'a>) -> Self {
        Self { stack }
    }

    /// Fetches firmware metadata from /dfu/info endpoint.
    ///
    /// Returns size and checksum needed before starting OTA flash writes.
    pub async fn get_firmware_info(&self) -> Result<FirmwareInfo, OtaError> {
        let port: u16 = OTA_PORT.parse().unwrap_or(8080);
        info!("Fetching firmware info from {}:{}/dfu/info", OTA_HOST, port);

        // Create TCP client
        static mut TCP_BUFS: TcpClientState<1, 512, 256> = TcpClientState::new();
        let tcp = unsafe { TcpClient::new(*self.stack, &*core::ptr::addr_of!(TCP_BUFS)) };

        // Create HTTP client with DNS
        let dns = DnsSocket::new(*self.stack);
        let mut client = HttpClient::new(&tcp, &dns);

        // Build URL
        let mut url_buf = [0u8; 64];
        let url_len = build_url(&mut url_buf, OTA_HOST, port, "/dfu/info");
        let url = core::str::from_utf8(&url_buf[..url_len]).unwrap_or("");

        // Make GET request
        let mut rx_buf = [0u8; 256];
        let mut request = client.request(Method::GET, url).await.map_err(|_| {
            error!("Failed to create info request");
            OtaError::TransportError
        })?;

        let response = request.send(&mut rx_buf).await.map_err(|_| {
            error!("Failed to send info request");
            OtaError::TransportError
        })?;

        if response.status.0 != 200 {
            error!("Info request failed with status {}", response.status.0);
            return Err(OtaError::TransportError);
        }

        // Read response body (JSON)
        let body = response.body().read_to_end().await.map_err(|_| {
            error!("Failed to read info response");
            OtaError::TransportError
        })?;

        // Parse JSON manually (no serde_json in no_std)
        // Expected: {"size":100,"checksum":12345,"version":"2.0.0"}
        let body_str = core::str::from_utf8(body).map_err(|_| OtaError::InvalidFirmware)?;
        let info = parse_firmware_info(body_str)?;

        info!(
            "Firmware info: size={}, checksum={:#x}",
            info.size, info.checksum
        );
        Ok(info)
    }

    /// Checks OTA server for firmware updates.
    ///
    /// Sends device status to /dfu endpoint and parses Command response.
    ///
    /// # Returns
    /// OtaCheckResult indicating if update is available
    pub async fn check_for_update(&mut self) -> Result<OtaCheckResult, OtaError> {
        let port: u16 = OTA_PORT.parse().unwrap_or(8080);
        info!("Checking for OTA updates at {}:{}/dfu", OTA_HOST, port);
        info!(
            "Current firmware version: {}",
            core::str::from_utf8(FIRMWARE_VERSION).unwrap_or("?")
        );

        // Create Status message using drogue-ajour-protocol
        let status = Status::first(FIRMWARE_VERSION, Some(MAX_CHUNK_SIZE as u32), None);

        // Send status and get command
        let command = self.send_status(&status, port).await?;

        // Process command
        match command {
            OtaCommand::Sync { version } => {
                let ver = core::str::from_utf8(&version).unwrap_or("?");
                info!("Server sync: version={}", ver);
                if version.as_slice() == FIRMWARE_VERSION {
                    info!("Firmware is up to date");
                    Ok(OtaCheckResult::UpToDate)
                } else {
                    info!("New version available: {}", ver);
                    // Need to request the actual update
                    Ok(OtaCheckResult::UpToDate) // For now, just report up to date
                }
            }
            OtaCommand::Write {
                version,
                offset,
                data,
            } => {
                let ver = core::str::from_utf8(&version).unwrap_or("?");
                info!(
                    "Update available: version={}, first chunk at offset={}, size={}",
                    ver,
                    offset,
                    data.len()
                );
                // Return that update is available - actual download handled separately
                Ok(OtaCheckResult::UpdateAvailable {
                    size: data.len() as u32,
                    checksum: 0, // Will be provided in Swap command
                })
            }
            OtaCommand::Swap { version, checksum } => {
                let ver = core::str::from_utf8(&version).unwrap_or("?");
                info!("Swap command received: version={}", ver);
                // Parse checksum (assuming it's a 4-byte big-endian CRC32)
                let crc = if checksum.len() >= 4 {
                    u32::from_be_bytes([checksum[0], checksum[1], checksum[2], checksum[3]])
                } else {
                    0
                };
                Ok(OtaCheckResult::UpdateAvailable {
                    size: 0,
                    checksum: crc,
                })
            }
            OtaCommand::Wait { poll } => {
                info!("Server says wait {} seconds", poll);
                Ok(OtaCheckResult::UpToDate)
            }
        }
    }

    /// Downloads firmware from OTA server in chunks.
    ///
    /// Implements full drogue-ajour protocol download loop:
    /// 1. Send Status::first() to start download
    /// 2. Receive Write commands with chunks
    /// 3. Send Status::update() with new offset
    /// 4. Repeat until Swap command received
    ///
    /// # Arguments
    /// * `on_chunk` - Callback for each received chunk (offset, data)
    ///
    /// # Returns
    /// CRC32 checksum from Swap command for verification
    pub async fn download_firmware<F>(&mut self, mut on_chunk: F) -> Result<u32, OtaError>
    where
        F: FnMut(u32, &[u8]),
    {
        let port: u16 = OTA_PORT.parse().unwrap_or(8080);
        info!("Starting firmware download from {}:{}/dfu", OTA_HOST, port);

        // Send initial Status to start download
        let status = Status::first(FIRMWARE_VERSION, Some(MAX_CHUNK_SIZE as u32), None);
        let mut command = self.send_status(&status, port).await?;

        let mut current_offset: u32 = 0;
        let mut target_version: Vec<u8> = Vec::new();

        loop {
            match command {
                OtaCommand::Write {
                    version,
                    offset,
                    data,
                } => {
                    info!("Received chunk: offset={}, size={}", offset, data.len());

                    // Store target version for subsequent requests
                    if target_version.is_empty() {
                        target_version = version.clone();
                    }

                    // Call chunk handler
                    on_chunk(offset, &data);

                    // Update offset
                    current_offset = offset + data.len() as u32;

                    // Send Status::update with new offset to request next chunk
                    let update_status = Status::update(
                        FIRMWARE_VERSION,
                        Some(MAX_CHUNK_SIZE as u32),
                        current_offset,
                        &target_version,
                        None,
                    );
                    command = self.send_status(&update_status, port).await?;
                }
                OtaCommand::Swap { checksum, .. } => {
                    info!("Download complete! Total bytes: {}", current_offset);
                    // Parse checksum
                    let crc = if checksum.len() >= 4 {
                        u32::from_be_bytes([checksum[0], checksum[1], checksum[2], checksum[3]])
                    } else {
                        0
                    };
                    info!("Expected CRC32: {:#x}", crc);
                    return Ok(crc);
                }
                OtaCommand::Sync { .. } => {
                    info!("Received Sync during download - firmware up to date");
                    return Err(OtaError::InvalidFirmware);
                }
                OtaCommand::Wait { poll } => {
                    info!("Server says wait {} seconds", poll);
                    return Err(OtaError::TransportError);
                }
            }
        }
    }

    /// Sends Status message and receives Command response.
    async fn send_status(&self, status: &Status<'_>, port: u16) -> Result<OtaCommand, OtaError> {
        // Serialize to CBOR
        let cbor_data: Vec<u8> = serde_cbor::to_vec(status).map_err(|_| {
            error!("Failed to serialize Status to CBOR");
            OtaError::TransportError
        })?;

        info!("Status CBOR size: {} bytes", cbor_data.len());

        // Create TCP client - smaller buffers for OTA check (not download)
        static mut TCP_BUFS: TcpClientState<1, 1024, 256> = TcpClientState::new();
        let tcp = unsafe { TcpClient::new(*self.stack, &*core::ptr::addr_of!(TCP_BUFS)) };

        // Create HTTP client with DNS
        let dns = DnsSocket::new(*self.stack);
        let mut client = HttpClient::new(&tcp, &dns);

        // Build URL
        let mut url_buf = [0u8; 64];
        let url_len = build_url(&mut url_buf, OTA_HOST, port, "/dfu");
        let url = core::str::from_utf8(&url_buf[..url_len]).unwrap_or("");

        // Make POST request - 1KB buffer for OTA check response (not download)
        let mut rx_buf = [0u8; 1024];
        let request = client.request(Method::POST, url).await.map_err(|_| {
            error!("Failed to create HTTP request");
            OtaError::TransportError
        })?;

        let request_with_body = request.body(cbor_data.as_slice());
        let mut request_with_content_type =
            request_with_body.content_type(reqwless::headers::ContentType::ApplicationOctetStream);

        let response = request_with_content_type
            .send(&mut rx_buf)
            .await
            .map_err(|_| {
                error!("Failed to send HTTP request");
                OtaError::TransportError
            })?;

        info!("OTA server response: {}", response.status.0);

        if response.status.0 != 200 {
            error!("OTA check failed with status {}", response.status.0);
            return Err(OtaError::TransportError);
        }

        // Read response body
        let body = response.body().read_to_end().await.map_err(|_| {
            error!("Failed to read response body");
            OtaError::TransportError
        })?;

        info!("Response body size: {} bytes", body.len());

        // Deserialize Command from CBOR and convert to owned type
        let command: Command<'_> = serde_cbor::from_slice(body).map_err(|_| {
            error!("Failed to deserialize Command from CBOR");
            OtaError::InvalidFirmware
        })?;

        Ok(OtaCommand::from_command(&command))
    }
}

/// Owned version of drogue-ajour Command for use after deserialization.
#[derive(Debug)]
#[allow(clippy::missing_docs_in_private_items)]
pub enum OtaCommand {
    /// Wait for specified seconds before retry.
    Wait { poll: u32 },
    /// Server confirms current version is synced.
    Sync { version: Vec<u8> },
    /// Write firmware chunk at offset.
    Write {
        version: Vec<u8>,
        offset: u32,
        data: Vec<u8>,
    },
    /// Swap to new firmware with checksum verification.
    Swap { version: Vec<u8>, checksum: Vec<u8> },
}

impl OtaCommand {
    /// Converts borrowed Command to owned OtaCommand.
    fn from_command(cmd: &Command<'_>) -> Self {
        match cmd {
            Command::Wait { poll, .. } => OtaCommand::Wait {
                poll: poll.unwrap_or(60),
            },
            Command::Sync { version, .. } => OtaCommand::Sync {
                version: version.to_vec(),
            },
            Command::Write {
                version,
                offset,
                data,
                ..
            } => OtaCommand::Write {
                version: version.to_vec(),
                offset: *offset,
                data: data.to_vec(),
            },
            Command::Swap {
                version, checksum, ..
            } => OtaCommand::Swap {
                version: version.to_vec(),
                checksum: checksum.to_vec(),
            },
        }
    }
}

/// Builds URL string into buffer, returns length written.
fn build_url(buf: &mut [u8], host: &str, port: u16, path: &str) -> usize {
    let mut pos = 0;

    // Write "http://"
    let prefix = b"http://";
    buf[pos..pos + prefix.len()].copy_from_slice(prefix);
    pos += prefix.len();

    // Write host
    let host_bytes = host.as_bytes();
    buf[pos..pos + host_bytes.len()].copy_from_slice(host_bytes);
    pos += host_bytes.len();

    // Write ":"
    buf[pos] = b':';
    pos += 1;

    // Write port (simple int to string)
    let mut port_buf = [0u8; 5];
    let port_len = write_u16(port, &mut port_buf);
    buf[pos..pos + port_len].copy_from_slice(&port_buf[..port_len]);
    pos += port_len;

    // Write path
    let path_bytes = path.as_bytes();
    buf[pos..pos + path_bytes.len()].copy_from_slice(path_bytes);
    pos += path_bytes.len();

    pos
}

/// Writes u16 to buffer, returns length.
fn write_u16(mut n: u16, buf: &mut [u8]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }

    let mut len = 0;
    let mut temp = [0u8; 5];
    while n > 0 {
        temp[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }

    // Reverse
    for i in 0..len {
        buf[i] = temp[len - 1 - i];
    }
    len
}

/// Parses firmware info JSON response.
///
/// Expected format: {"size":100,"checksum":12345,"version":"2.0.0"}
fn parse_firmware_info(json: &str) -> Result<FirmwareInfo, OtaError> {
    // Simple JSON parsing without serde
    let size = extract_json_u32(json, "size").ok_or(OtaError::InvalidFirmware)?;
    let checksum = extract_json_u32(json, "checksum").ok_or(OtaError::InvalidFirmware)?;
    let version = extract_json_string(json, "version")
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_default();

    Ok(FirmwareInfo {
        size,
        checksum,
        version,
    })
}

/// Extracts u32 value from JSON by key.
fn extract_json_u32(json: &str, key: &str) -> Option<u32> {
    let pattern = alloc::format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

/// Extracts string value from JSON by key.
fn extract_json_string<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = alloc::format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

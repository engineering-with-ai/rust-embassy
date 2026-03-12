//! Mock OTA server implementing Drogue Ajour protocol.
//!
//! Serves firmware via HTTP POST /dfu endpoint using CBOR encoding.
//! Extended with /dfu/info endpoint for upfront metadata (size, checksum).

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use drogue_ajour_protocol::{Command, Status};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Server mode for different test scenarios.
#[derive(Clone, Debug)]
pub enum ServerMode {
    /// Always respond with Sync (device up to date).
    SyncOnly,
    /// Serve firmware in chunks via Write commands.
    ServeFirmware {
        /// Firmware binary data.
        firmware: Arc<Vec<u8>>,
        /// Chunk size for Write commands.
        chunk_size: usize,
    },
}

/// OTA server state tracking requests.
#[derive(Clone)]
pub struct OtaServerState {
    /// Target version string.
    target_version: Arc<Vec<u8>>,
    /// Counts requests received.
    request_count: Arc<RwLock<u32>>,
    /// Current offset in firmware transfer.
    current_offset: Arc<RwLock<u32>>,
    /// Server operating mode.
    mode: ServerMode,
    /// Last reported device version.
    reported_version: Arc<RwLock<Option<Vec<u8>>>>,
}

impl OtaServerState {
    /// Creates new server state in SyncOnly mode.
    pub fn new(target_version: &str) -> Self {
        Self {
            target_version: Arc::new(target_version.as_bytes().to_vec()),
            request_count: Arc::new(RwLock::new(0)),
            current_offset: Arc::new(RwLock::new(0)),
            mode: ServerMode::SyncOnly,
            reported_version: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates server state that serves firmware.
    pub fn with_firmware(target_version: &str, firmware: Vec<u8>, chunk_size: usize) -> Self {
        Self {
            target_version: Arc::new(target_version.as_bytes().to_vec()),
            request_count: Arc::new(RwLock::new(0)),
            current_offset: Arc::new(RwLock::new(0)),
            mode: ServerMode::ServeFirmware {
                firmware: Arc::new(firmware),
                chunk_size,
            },
            reported_version: Arc::new(RwLock::new(None)),
        }
    }

    /// Gets number of requests received.
    pub async fn get_request_count(&self) -> u32 {
        *self.request_count.read().await
    }

    /// Gets current firmware transfer offset.
    pub async fn get_current_offset(&self) -> u32 {
        *self.current_offset.read().await
    }

    /// Gets the last reported device version.
    pub async fn get_reported_version(&self) -> Option<String> {
        self.reported_version
            .read()
            .await
            .as_ref()
            .map(|v| String::from_utf8_lossy(v).to_string())
    }
}

/// Handles POST /dfu requests from device.
///
/// Protocol: Device always sends Status. The Status.update field contains
/// UpdateStatus with offset when reporting progress after Write commands.
async fn handle_dfu(State(state): State<OtaServerState>, body: Bytes) -> Result<Bytes, StatusCode> {
    // Increment request counter
    *state.request_count.write().await += 1;

    // Parse Status message (device always sends Status per protocol)
    let status: Status = serde_cbor::from_slice(&body).map_err(|e| {
        eprintln!("[OTA Server] Failed to parse Status: {:?}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Extract version and optional update offset from Status.update field
    let version = status.version.to_vec();
    let update_offset = status.update.as_ref().map(|u| u.offset);

    // Track reported version
    *state.reported_version.write().await = Some(version.clone());

    println!(
        "[OTA Server] Received Status: version={:?}, update_offset={:?}",
        String::from_utf8_lossy(&version),
        update_offset
    );

    // Generate and serialize response based on mode
    let response_bytes = match &state.mode {
        ServerMode::SyncOnly => {
            let cmd = Command::new_sync(&state.target_version, Some(300), None);
            serde_cbor::to_vec(&cmd).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        ServerMode::ServeFirmware {
            firmware,
            chunk_size,
        } => {
            let offset = update_offset.unwrap_or(0);
            *state.current_offset.write().await = offset;

            if offset as usize >= firmware.len() {
                // All chunks sent, send Swap with checksum
                let checksum = crc32_checksum(firmware);
                let checksum_bytes = checksum.to_be_bytes();
                println!("[OTA Server] Sending Swap: checksum={:#x}", checksum);
                let cmd = Command::new_swap(&state.target_version, &checksum_bytes, None);
                serde_cbor::to_vec(&cmd).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            } else {
                // Send next chunk
                let end = std::cmp::min(offset as usize + chunk_size, firmware.len());
                let chunk = &firmware[offset as usize..end];
                println!(
                    "[OTA Server] Sending Write: offset={}, size={}",
                    offset,
                    chunk.len()
                );
                let cmd = Command::new_write(&state.target_version, offset, chunk, None);
                serde_cbor::to_vec(&cmd).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            }
        }
    };

    Ok(Bytes::from(response_bytes))
}

/// Computes CRC32 checksum of data.
fn crc32_checksum(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Handles GET /dfu/info - returns firmware metadata as JSON.
async fn handle_info(State(state): State<OtaServerState>) -> impl IntoResponse {
    match &state.mode {
        ServerMode::SyncOnly => (StatusCode::NOT_FOUND, String::new()),
        ServerMode::ServeFirmware { firmware, .. } => {
            let checksum = crc32_checksum(firmware);
            let version = String::from_utf8_lossy(&state.target_version).to_string();
            println!(
                "[OTA Server] Info request: size={}, checksum={}, version={}",
                firmware.len(),
                checksum,
                version
            );
            let json = format!(
                r#"{{"size":{},"checksum":{},"version":"{}"}}"#,
                firmware.len(),
                checksum,
                version
            );
            (StatusCode::OK, json)
        }
    }
}

/// Creates the OTA server router.
fn create_router(state: OtaServerState) -> Router {
    Router::new()
        .route("/dfu", post(handle_dfu))
        .route("/dfu/info", get(handle_info))
        .with_state(state)
}

/// Starts the OTA server, returns the port and state.
pub async fn start_server(
    target_version: &str,
) -> (u16, OtaServerState, tokio::task::JoinHandle<()>) {
    let state = OtaServerState::new(target_version);
    start_server_with_state(state).await
}

/// Starts the OTA server with firmware to serve.
pub async fn start_server_with_firmware(
    target_version: &str,
    firmware: Vec<u8>,
    chunk_size: usize,
) -> (u16, OtaServerState, tokio::task::JoinHandle<()>) {
    let state = OtaServerState::with_firmware(target_version, firmware, chunk_size);
    start_server_with_state(state).await
}

/// Starts the OTA server with custom state.
async fn start_server_with_state(
    state: OtaServerState,
) -> (u16, OtaServerState, tokio::task::JoinHandle<()>) {
    let router = create_router(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    println!("[OTA Server] Starting on port {}", port);

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (port, state, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_responds_sync_to_status() {
        // Arrange
        let (port, state, _handle) = start_server("1.0.0").await;

        // Act - device reports its version
        let status = Status::first(b"1.0.0", Some(4096), None);
        let body = serde_cbor::to_vec(&status).unwrap();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://localhost:{}/dfu", port))
            .body(body)
            .send()
            .await
            .unwrap();

        // Assert - should get Sync command
        assert!(response.status().is_success());
        let response_body = response.bytes().await.unwrap();
        let cmd: Command = serde_cbor::from_slice(&response_body).unwrap();
        assert!(matches!(cmd, Command::Sync { .. }));
        assert_eq!(state.get_request_count().await, 1);
    }

    #[tokio::test]
    async fn test_server_sends_write_then_swap() {
        // Arrange - 100 byte firmware, 50 byte chunks
        let firmware = vec![0xAB; 100];
        let (port, state, _handle) = start_server_with_firmware("2.0.0", firmware, 50).await;
        let client = reqwest::Client::new();

        // Act 1 - initial Status request
        let status = Status::first(b"1.0.0", Some(4096), None);
        let response = client
            .post(format!("http://localhost:{}/dfu", port))
            .body(serde_cbor::to_vec(&status).unwrap())
            .send()
            .await
            .unwrap();

        // Assert 1 - should get Write at offset 0
        let body1 = response.bytes().await.unwrap();
        let cmd: Command = serde_cbor::from_slice(&body1).unwrap();
        match cmd {
            Command::Write { offset, data, .. } => {
                assert_eq!(offset, 0);
                assert_eq!(data.len(), 50);
            }
            _ => panic!("Expected Write command"),
        }

        // Act 2 - Status with update progress after writing first chunk
        // Protocol: Device sends Status with embedded update field containing offset
        let status = Status::update(b"1.0.0", Some(4096), 50, b"2.0.0", None);
        let response = client
            .post(format!("http://localhost:{}/dfu", port))
            .body(serde_cbor::to_vec(&status).unwrap())
            .send()
            .await
            .unwrap();

        // Assert 2 - should get Write at offset 50
        let body2 = response.bytes().await.unwrap();
        let cmd: Command = serde_cbor::from_slice(&body2).unwrap();
        match cmd {
            Command::Write { offset, data, .. } => {
                assert_eq!(offset, 50);
                assert_eq!(data.len(), 50);
            }
            _ => panic!("Expected Write command"),
        }

        // Act 3 - Status with update progress after writing second chunk (100 bytes total)
        let status = Status::update(b"1.0.0", Some(4096), 100, b"2.0.0", None);
        let response = client
            .post(format!("http://localhost:{}/dfu", port))
            .body(serde_cbor::to_vec(&status).unwrap())
            .send()
            .await
            .unwrap();

        // Assert 3 - should get Swap with checksum
        let body3 = response.bytes().await.unwrap();
        let cmd: Command = serde_cbor::from_slice(&body3).unwrap();
        assert!(matches!(cmd, Command::Swap { .. }));
        assert_eq!(state.get_request_count().await, 3);
    }
}

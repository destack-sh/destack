use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::protocol::{MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolRange};

use super::{Service, ServiceError};

/// Metadata stored for a running workspace service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Metadata schema version.
    pub schema_version: u32,
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Stable command instance id.
    pub instance_id: String,
    /// Socket path for workspace connections.
    pub socket_path: PathBuf,
    /// Local WebSocket URL for browser workspace connections.
    pub websocket_url: String,
    /// Workspace server process id.
    pub pid: u32,
    /// Workspace protocol range.
    pub protocol: ProtocolRange,
    /// Workspace server version string.
    pub version: String,
    /// Unix timestamp for workspace server start.
    pub started_at: u64,
}

impl Metadata {
    /// Create metadata for a workspace service.
    pub fn new(
        service: &Service,
        websocket_addr: SocketAddr,
        websocket_token: &str,
    ) -> Result<Self, ServiceError> {
        let started_at = Self::current_unix_seconds()?;

        Ok(Self {
            schema_version: 1,
            home: service.home.clone(),
            instance_id: service.instance_id.clone(),
            socket_path: service.socket_path.clone(),
            websocket_url: service.websocket_url(websocket_addr, websocket_token),
            pid: std::process::id(),
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at,
        })
    }

    /// Return the current unix timestamp in seconds.
    fn current_unix_seconds() -> Result<u64, ServiceError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(ServiceError::Time)?;

        Ok(duration.as_secs())
    }
}

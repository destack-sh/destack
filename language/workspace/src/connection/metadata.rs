use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::protocol::{MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolRange};

use super::{WorkspaceEndpoint, WorkspaceEndpointError};

/// Metadata stored for a running workspace endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceServerMetadata {
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

impl WorkspaceServerMetadata {
    /// Create metadata for a workspace endpoint.
    pub fn new(
        endpoint: &WorkspaceEndpoint,
        websocket_addr: SocketAddr,
        websocket_token: &str,
    ) -> Result<Self, WorkspaceEndpointError> {
        let started_at = Self::current_unix_seconds()?;

        Ok(Self {
            schema_version: 1,
            home: endpoint.home.clone(),
            instance_id: endpoint.instance_id.clone(),
            socket_path: endpoint.socket_path.clone(),
            websocket_url: endpoint.websocket_url(websocket_addr, websocket_token),
            pid: std::process::id(),
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at,
        })
    }

    /// Return the current unix timestamp in seconds.
    fn current_unix_seconds() -> Result<u64, WorkspaceEndpointError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(WorkspaceEndpointError::Time)?;

        Ok(duration.as_secs())
    }
}

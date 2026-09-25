use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tspp_artifact::BuildId;
use tspp_rpc::ProtocolVersion;

use super::{DaemonEndpoint, DaemonEndpointError};

/// Discovery record for one running daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMetadata {
    /// Metadata schema version.
    pub schema_version: u32,
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Stable endpoint instance identifier.
    pub instance_id: String,
    /// Local RPC socket path.
    pub socket_path: PathBuf,
    /// Authenticated browser WebSocket URL.
    pub websocket_url: String,
    /// Daemon process identifier.
    pub process_id: u32,
    /// Exact RPC wire grammar.
    pub rpc_version: u16,
    /// TS++ toolchain build identity.
    pub build_id: String,
    /// Daemon package version.
    pub version: String,
    /// Unix timestamp when the daemon started.
    pub started_at: u64,
}

impl DaemonMetadata {
    /// Create one discovery record.
    pub fn new(
        endpoint: &DaemonEndpoint,
        websocket_address: SocketAddr,
        websocket_token: &str,
        build_id: BuildId,
    ) -> Result<Self, DaemonEndpointError> {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(DaemonEndpointError::Time)?
            .as_secs();
        let build_id = build_id.to_string();

        Ok(Self {
            schema_version: 1,
            home: endpoint.home.clone(),
            instance_id: endpoint.instance_id.clone(),
            socket_path: endpoint.socket_path.clone(),
            websocket_url: endpoint.websocket_url(websocket_address, websocket_token),
            process_id: process::id(),
            rpc_version: ProtocolVersion::CURRENT.0,
            build_id,
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at,
        })
    }
}

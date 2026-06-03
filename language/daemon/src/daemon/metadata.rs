use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::protocol::{MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolRange};

use super::DaemonEndpoint;

/// Metadata stored for a running daemon endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMetadata {
    /// Metadata schema version.
    pub schema_version: u32,
    /// Machine-local Destack home.
    pub home: PathBuf,
    /// Stable toolchain instance id.
    pub instance_id: String,
    /// Socket path for daemon connections.
    pub socket_path: PathBuf,
    /// Daemon process id.
    pub pid: u32,
    /// Daemon protocol range.
    pub protocol: ProtocolRange,
    /// Daemon version string.
    pub version: String,
    /// Unix timestamp for daemon start.
    pub started_at: u64,
}

impl DaemonMetadata {
    /// Create metadata for a daemon endpoint.
    pub fn new(endpoint: &DaemonEndpoint) -> Self {
        Self {
            schema_version: 1,
            home: endpoint.home.clone(),
            instance_id: endpoint.instance_id.clone(),
            socket_path: endpoint.socket_path.clone(),
            pid: std::process::id(),
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: current_unix_seconds(),
        }
    }
}

/// Return the current unix timestamp in seconds.
fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

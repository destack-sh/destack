use serde::{Deserialize, Serialize};

use super::{ProtocolRange, ProtocolVersion, RepositoryId};

/// Default maximum frame size in bytes.
pub const DEFAULT_MAX_FRAME_BYTES: u64 = 64 * 1024 * 1024;
/// Default maximum protocol payload size in bytes.
pub const DEFAULT_MAX_PAYLOAD_BYTES: u64 = DEFAULT_MAX_FRAME_BYTES;

/// Negotiated protocol limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    /// Maximum frame size in bytes.
    pub max_frame_bytes: u64,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: u64,
}

impl ProtocolLimits {
    /// Create a new protocol limits payload.
    pub const fn new(max_frame_bytes: u64, max_payload_bytes: u64) -> Self {
        Self {
            max_frame_bytes,
            max_payload_bytes,
        }
    }

    /// Negotiate the minimum limits between peers.
    pub fn negotiate(&self, other: &Self) -> Self {
        // select the strictest limits
        Self {
            max_frame_bytes: self.max_frame_bytes.min(other.max_frame_bytes),
            max_payload_bytes: self.max_payload_bytes.min(other.max_payload_bytes),
        }
    }
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
        }
    }
}

/// Client descriptor sent during handshake negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDescriptor {
    /// Client name (cli, lsp, editor, etc.).
    pub name: String,
    /// Client version string.
    pub version: String,
    /// Optional build identifier.
    pub build: Option<String>,
    /// Optional process id.
    pub pid: Option<u32>,
}

/// Server descriptor sent during handshake negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerDescriptor {
    /// Server name.
    pub name: String,
    /// Server version string.
    pub version: String,
    /// Optional build identifier.
    pub build: Option<String>,
    /// Optional process id.
    pub pid: Option<u32>,
}

/// Handshake request payload for protocol negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// Supported protocol range on the client.
    pub protocol: ProtocolRange,
    /// Client descriptor.
    pub client: ClientDescriptor,
    /// Client protocol limits.
    pub limits: ProtocolLimits,
}

/// Handshake response payload for protocol negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Selected protocol version.
    pub protocol: ProtocolVersion,
    /// Server descriptor.
    pub server: ServerDescriptor,
    /// Negotiated protocol limits.
    pub limits: ProtocolLimits,
    /// Allocated daemon session id.
    pub session_id: RepositoryId,
}

#[cfg(test)]
mod tests {
    use super::ProtocolLimits;

    #[test]
    fn test_protocol_limits_negotiate() {
        // build two limit sets
        let a = ProtocolLimits::new(1024, 1024);
        let b = ProtocolLimits::new(2048, 512);

        // negotiate the strictest limits
        let negotiated = a.negotiate(&b);

        // assert the negotiated values
        assert_eq!(negotiated.max_frame_bytes, 1024);
        assert_eq!(negotiated.max_payload_bytes, 512);
    }
}

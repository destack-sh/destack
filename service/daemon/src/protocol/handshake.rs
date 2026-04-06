use serde::{Deserialize, Serialize};

use super::{ProtocolRange, ProtocolVersion, RepositoryId};

/// Negotiated protocol limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolLimits {
    /// Maximum frame size in bytes.
    pub max_frame_bytes: u64,
    /// Maximum payload size in bytes.
    pub max_payload_bytes: u64,
    /// Maximum inflight requests.
    pub max_inflight_requests: u32,
    /// Maximum inflight notifications.
    pub max_inflight_notifications: u32,
}

impl ProtocolLimits {
    /// Create a new protocol limits payload.
    pub const fn new(
        max_frame_bytes: u64,
        max_payload_bytes: u64,
        max_inflight_requests: u32,
        max_inflight_notifications: u32,
    ) -> Self {
        Self {
            max_frame_bytes,
            max_payload_bytes,
            max_inflight_requests,
            max_inflight_notifications,
        }
    }

    /// Negotiate the minimum limits between peers.
    pub fn negotiate(&self, other: &Self) -> Self {
        // select the strictest limits
        Self {
            max_frame_bytes: self.max_frame_bytes.min(other.max_frame_bytes),
            max_payload_bytes: self.max_payload_bytes.min(other.max_payload_bytes),
            max_inflight_requests: self.max_inflight_requests.min(other.max_inflight_requests),
            max_inflight_notifications: self
                .max_inflight_notifications
                .min(other.max_inflight_notifications),
        }
    }
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 64 * 1024 * 1024,
            max_payload_bytes: 64 * 1024 * 1024,
            max_inflight_requests: 1024,
            max_inflight_notifications: 4096,
        }
    }
}

/// Client metadata for handshake negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name (cli, lsp, editor, etc.).
    pub name: String,
    /// Client version string.
    pub version: String,
    /// Optional build identifier.
    pub build: Option<String>,
    /// Optional process id.
    pub pid: Option<u32>,
}

/// Server metadata for handshake negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
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
    /// Client metadata.
    pub client: ClientInfo,
    /// Client protocol limits.
    pub limits: ProtocolLimits,
}

/// Handshake response payload for protocol negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// Selected protocol version.
    pub protocol: ProtocolVersion,
    /// Server metadata.
    pub server: ServerInfo,
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
        let a = ProtocolLimits::new(1024, 1024, 8, 16);
        let b = ProtocolLimits::new(2048, 512, 4, 32);

        // negotiate the strictest limits
        let negotiated = a.negotiate(&b);

        // assert the negotiated values
        assert_eq!(negotiated.max_frame_bytes, 1024);
        assert_eq!(negotiated.max_payload_bytes, 512);
        assert_eq!(negotiated.max_inflight_requests, 4);
        assert_eq!(negotiated.max_inflight_notifications, 16);
    }
}

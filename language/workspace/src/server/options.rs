use crate::protocol::{
    MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ProtocolLimits, ProtocolRange, ServerDescriptor,
};

/// Options for initializing a protocol server.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Supported protocol range.
    pub protocol: ProtocolRange,
    /// Limits advertised by the server.
    pub limits: ProtocolLimits,
    /// Server descriptor for the handshake response.
    pub server: ServerDescriptor,
}

impl Default for ServerOptions {
    /// Return default protocol server options.
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            server: ServerDescriptor {
                name: "destack-workspace".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
            },
        }
    }
}

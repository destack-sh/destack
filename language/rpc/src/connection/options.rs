use crate::protocol::HandshakeRequest;
use crate::{Code, Limits, Peer, ProtocolVersion, Status};

/// Options shared by both sides of an RPC connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionOptions {
    /// Supported exact grammar versions in preference order.
    pub versions: Vec<ProtocolVersion>,
    /// Advertised resource limits.
    pub limits: Limits,
    /// Informational peer description.
    pub peer: Peer,
}

impl ConnectionOptions {
    /// Create default options for one named peer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            versions: vec![ProtocolVersion::CURRENT],
            limits: Limits::default(),
            peer: Peer {
                name: name.into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build_id: None,
            },
        }
    }

    /// Negotiate one peer request against these options.
    pub(crate) fn negotiate(
        &self,
        request: &HandshakeRequest,
    ) -> Result<(ProtocolVersion, Limits), Status> {
        let Some(version) = ProtocolVersion::negotiate(&request.versions, &self.versions) else {
            return Err(Status::new(
                Code::FailedPrecondition,
                format!(
                    "RPC versions do not overlap: local {:?}, remote {:?}",
                    self.versions, request.versions
                ),
            ));
        };
        request.limits.validate()?;

        let limits = self.limits.negotiate(request.limits);
        limits.validate()?;

        Ok((version, limits))
    }
}

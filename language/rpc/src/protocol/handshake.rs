use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{Code, CodecError, ProtocolVersion, Status};
use crate::{
    ConnectionError, MethodFingerprint, MethodId, ServiceFingerprint, ServiceId, Transport,
};

/// Frozen connection negotiation message independent of versioned call messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum Handshake {
    /// Request one connection.
    Request(HandshakeRequest),
    /// Accept or reject one connection.
    Response(HandshakeResponse),
}

/// Encoder and decoder for the frozen connection negotiation grammar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HandshakeCodec {
    /// Largest encoded negotiation message in bytes.
    max_message_bytes: usize,
}

impl HandshakeCodec {
    /// Create one negotiation codec with an exact byte limit.
    pub(crate) const fn new(max_message_bytes: usize) -> Self {
        Self { max_message_bytes }
    }

    /// Encode one connection negotiation message.
    pub(crate) fn encode(self, handshake: &Handshake) -> Result<Vec<u8>, CodecError> {
        let bytes = tspp_serde::to_vec(handshake).map_err(CodecError::Encode)?;
        if bytes.len() > self.max_message_bytes {
            return Err(CodecError::TooLarge {
                limit: self.max_message_bytes,
                actual: bytes.len(),
            });
        }

        Ok(bytes)
    }

    /// Decode one connection negotiation message.
    pub(crate) fn decode(self, bytes: &[u8]) -> Result<Handshake, CodecError> {
        if bytes.len() > self.max_message_bytes {
            return Err(CodecError::TooLarge {
                limit: self.max_message_bytes,
                actual: bytes.len(),
            });
        }

        tspp_serde::from_slice(bytes).map_err(CodecError::Decode)
    }

    /// Encode and send one connection handshake.
    pub(crate) fn send(
        self,
        transport: &dyn Transport,
        handshake: &Handshake,
    ) -> Result<(), ConnectionError> {
        let bytes = self.encode(handshake)?;
        transport.send(&bytes)?;

        Ok(())
    }

    /// Receive and decode one connection handshake.
    pub(crate) fn receive(self, transport: &dyn Transport) -> Result<Handshake, ConnectionError> {
        let bytes = transport.receive()?;
        let handshake = self.decode(&bytes)?;

        Ok(handshake)
    }
}

/// Negotiated RPC resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Limits {
    /// Largest encoded RPC message in bytes.
    pub max_message_bytes: u64,
    /// Largest assembled value payload in bytes.
    pub max_payload_bytes: u64,
    /// Largest number of concurrent calls in either direction.
    pub max_concurrent_calls: u32,
    /// Initial item window in each stream direction.
    pub stream_window: u32,
}

impl Limits {
    /// Select the strictest limit from two peers.
    pub fn negotiate(self, other: Self) -> Self {
        Self {
            max_message_bytes: self.max_message_bytes.min(other.max_message_bytes),
            max_payload_bytes: self.max_payload_bytes.min(other.max_payload_bytes),
            max_concurrent_calls: self.max_concurrent_calls.min(other.max_concurrent_calls),
            stream_window: self.stream_window.min(other.stream_window),
        }
    }

    /// Validate that every negotiated resource is usable.
    pub fn validate(self) -> Result<(), Status> {
        if self.max_message_bytes == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "message limit must be positive",
            ));
        }
        if self.max_payload_bytes == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "payload limit must be positive",
            ));
        }
        if self.max_concurrent_calls == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "concurrent call limit must be positive",
            ));
        }
        if self.stream_window == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "stream window must be positive",
            ));
        }

        Ok(())
    }

    /// Validate one limit selection returned by a peer.
    pub(crate) fn validate_selected(self, selected: Self) -> Result<(), Status> {
        selected.validate()?;

        let exceeds_local = selected.max_message_bytes > self.max_message_bytes
            || selected.max_payload_bytes > self.max_payload_bytes
            || selected.max_concurrent_calls > self.max_concurrent_calls
            || selected.stream_window > self.stream_window;
        if exceeds_local {
            return Err(Status::new(
                Code::FailedPrecondition,
                "selected RPC limits exceed the caller's advertised limits",
            ));
        }

        Ok(())
    }
}

impl Default for Limits {
    /// Return conservative RPC limits.
    fn default() -> Self {
        Self {
            max_message_bytes: 8 * 1024 * 1024,
            max_payload_bytes: 256 * 1024 * 1024,
            max_concurrent_calls: 1024,
            stream_window: 32,
        }
    }
}

/// Informational RPC peer description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Peer {
    /// Stable implementation name.
    pub name: String,
    /// Human-readable implementation version.
    pub version: String,
    /// Optional exact implementation build identity.
    pub build_id: Option<String>,
}

/// One method contract selected by an accepting peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MethodOffer {
    /// Offered method.
    pub method: MethodId,
    /// Selected exact method contract.
    pub fingerprint: MethodFingerprint,
}

/// One service offered by an accepting peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ServiceOffer {
    /// Offered service.
    pub service: ServiceId,
    /// Exact complete service schema.
    pub fingerprint: ServiceFingerprint,
    /// Selected required method contracts.
    pub methods: Vec<MethodOffer>,
}

/// RPC connection negotiation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct HandshakeRequest {
    /// Supported exact grammar versions in preference order.
    pub(crate) versions: Vec<ProtocolVersion>,
    /// Requested resource limits.
    pub(crate) limits: Limits,
    /// Connecting peer description.
    pub(crate) peer: Peer,
    /// Services requested from the accepting peer.
    pub(crate) services: Vec<ServiceId>,
}

/// RPC connection negotiation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum HandshakeResponse {
    /// Accept the connection with the negotiated grammar and limits.
    Accepted {
        /// Negotiated grammar version.
        version: ProtocolVersion,
        /// Negotiated resource limits.
        limits: Limits,
        /// Accepting peer description.
        peer: Peer,
        /// Services offered by the accepting peer.
        services: Vec<ServiceOffer>,
    },
    /// Reject the connection.
    Rejected {
        /// Accepting peer description.
        peer: Peer,
        /// Negotiation failure.
        status: Status,
    },
}

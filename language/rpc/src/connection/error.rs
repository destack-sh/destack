use crate::protocol::CodecError;
use crate::{MethodId, ServiceError, ServiceId, ServiceSchemaError, Status, TransportError};

/// Failure to construct a typed RPC client connection.
#[derive(Debug)]
pub enum ConnectError {
    /// The declared service schema is invalid.
    Schema(ServiceSchemaError),
    /// RPC connection negotiation failed.
    Connection(ConnectionError),
}

impl std::fmt::Display for ConnectError {
    /// Format this typed connection failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(error) => write!(formatter, "RPC service schema failed: {error}"),
            Self::Connection(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ConnectError {
    /// Return the underlying typed connection failure.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}

impl From<ServiceSchemaError> for ConnectError {
    /// Convert one service schema failure.
    fn from(error: ServiceSchemaError) -> Self {
        Self::Schema(error)
    }
}

impl From<ConnectionError> for ConnectError {
    /// Convert one connection negotiation failure.
    fn from(error: ConnectionError) -> Self {
        Self::Connection(error)
    }
}

/// Failure to establish or use one RPC connection.
#[derive(Debug)]
pub enum ConnectionError {
    /// RPC message encoding or decoding failed.
    Codec(CodecError),
    /// The underlying transport failed.
    Transport(TransportError),
    /// The peer rejected connection negotiation.
    Rejected(Status),
    /// A required service was not offered with its expected schema.
    ServiceNotOffered(ServiceId),
    /// A required method was not offered with its expected contract.
    MethodNotOffered {
        /// Required service.
        service: ServiceId,
        /// Required method.
        method: MethodId,
    },
    /// The peer violated the negotiated RPC protocol.
    Protocol(String),
    /// The connection is closed.
    Closed,
    /// Connection termination and transport closure both failed.
    Shutdown {
        /// Failure that required connection termination.
        failure: Box<ConnectionError>,
        /// Failure while closing the transport.
        close: Box<ConnectionError>,
    },
}

impl std::fmt::Display for ConnectionError {
    /// Format this connection failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => write!(formatter, "{error}"),
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::Rejected(status) => write!(formatter, "RPC connection rejected: {status}"),
            Self::ServiceNotOffered(service) => {
                write!(formatter, "RPC service {service:?} is not offered")
            }
            Self::MethodNotOffered { service, method } => {
                write!(
                    formatter,
                    "RPC method {method:?} of service {service:?} is not offered"
                )
            }
            Self::Protocol(message) => write!(formatter, "RPC protocol failed: {message}"),
            Self::Closed => write!(formatter, "RPC connection is closed"),
            Self::Shutdown { failure, close } => {
                write!(formatter, "{failure}; RPC transport close failed: {close}")
            }
        }
    }
}

impl std::error::Error for ConnectionError {
    /// Return the underlying connection failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Rejected(status) => Some(status),
            Self::Shutdown { failure, .. } => Some(failure),
            Self::ServiceNotOffered(_)
            | Self::MethodNotOffered { .. }
            | Self::Protocol(_)
            | Self::Closed => None,
        }
    }
}

impl From<CodecError> for ConnectionError {
    /// Convert one message codec failure.
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}

impl From<TransportError> for ConnectionError {
    /// Convert one transport failure.
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

/// Failure while serving one RPC connection.
#[derive(Debug)]
pub enum ServerError {
    /// Connection negotiation or message exchange failed.
    Connection(ConnectionError),
    /// A service call failed after the connection was established.
    Service(ServiceError),
    /// A native RPC receiver thread failed.
    Thread,
    /// Serving and transport closure both failed.
    Shutdown {
        /// Failure that terminated serving.
        failure: Box<ServerError>,
        /// Failure while closing the connection transport.
        close: ConnectionError,
    },
}

impl std::fmt::Display for ServerError {
    /// Format this server failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(error) => write!(formatter, "{error}"),
            Self::Service(error) => write!(formatter, "{error}"),
            Self::Thread => write!(formatter, "RPC receiver thread failed"),
            Self::Shutdown { failure, close } => {
                write!(formatter, "{failure}; RPC transport close failed: {close}")
            }
        }
    }
}

impl std::error::Error for ServerError {
    /// Return the underlying server failure.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connection(error) => Some(error),
            Self::Service(error) => Some(error),
            Self::Thread => None,
            Self::Shutdown { failure, .. } => Some(failure),
        }
    }
}

impl From<ConnectionError> for ServerError {
    /// Convert one connection failure.
    fn from(error: ConnectionError) -> Self {
        Self::Connection(error)
    }
}

impl From<ServiceError> for ServerError {
    /// Convert one service call failure.
    fn from(error: ServiceError) -> Self {
        Self::Service(error)
    }
}

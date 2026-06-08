use super::{
    PayloadReceiveError, ProtocolCodecError, ProtocolError, QueryPayloadCodecError, TransportError,
};

/// Errors returned by protocol clients.
#[derive(Debug)]
pub enum ClientError {
    /// Protocol codec error.
    Codec(ProtocolCodecError),
    /// Transport error.
    Transport(TransportError),
    /// Server returned an error response.
    Server(ProtocolError),
    /// Payload receive error.
    Payload(PayloadReceiveError),
    /// Query payload encoding or decoding error.
    QueryPayload(QueryPayloadCodecError),
    /// The response payload was unexpected.
    UnexpectedResponse(String),
}

impl std::fmt::Display for ClientError {
    /// Format the protocol client error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => write!(formatter, "protocol codec error: {error}"),
            Self::Transport(error) => write!(formatter, "transport error: {error}"),
            Self::Server(error) => write!(formatter, "server error: {error}"),
            Self::Payload(error) => write!(formatter, "payload error: {error}"),
            Self::QueryPayload(error) => write!(formatter, "query payload error: {error}"),
            Self::UnexpectedResponse(message) => {
                write!(formatter, "unexpected response: {message}")
            }
        }
    }
}

impl std::error::Error for ClientError {
    /// Return the underlying error source when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Server(error) => Some(error),
            Self::Payload(error) => Some(error),
            Self::QueryPayload(error) => Some(error),
            Self::UnexpectedResponse(_) => None,
        }
    }
}

impl From<TransportError> for ClientError {
    /// Convert a transport error into a client error.
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

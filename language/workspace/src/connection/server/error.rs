use crate::TransportError;
use crate::connection::PayloadSendError;
use crate::protocol::ProtocolCodecError;

/// Errors returned by protocol server loops.
#[derive(Debug)]
pub enum ServerError {
    /// Protocol codec error.
    Codec(ProtocolCodecError),
    /// Payload transfer error.
    Payload(PayloadSendError),
    /// Transport error.
    Transport(TransportError),
    /// Unexpected response message.
    UnexpectedResponse,
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Codec(error) => write!(f, "protocol codec error: {error}"),
            ServerError::Payload(error) => write!(f, "payload error: {error}"),
            ServerError::Transport(error) => write!(f, "transport error: {error}"),
            ServerError::UnexpectedResponse => {
                write!(f, "unexpected protocol response received by server")
            }
        }
    }
}

impl std::error::Error for ServerError {}

impl From<TransportError> for ServerError {
    fn from(error: TransportError) -> Self {
        ServerError::Transport(error)
    }
}

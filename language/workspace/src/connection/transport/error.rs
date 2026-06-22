use crate::protocol::FrameError;

/// Transport errors for framed payloads.
#[derive(Debug)]
pub enum TransportError {
    /// Frame codec error.
    Frame(FrameError),
    /// Io error.
    Io(std::io::Error),
    /// Transport is closed.
    Closed,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format transport errors
        match self {
            TransportError::Frame(error) => write!(f, "{error}"),
            TransportError::Io(error) => write!(f, "transport io error: {error}"),
            TransportError::Closed => write!(f, "transport closed"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<FrameError> for TransportError {
    fn from(error: FrameError) -> Self {
        TransportError::Frame(error)
    }
}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        TransportError::Io(error)
    }
}

/// Failure to exchange one transport message.
#[derive(Debug)]
pub enum TransportError {
    /// One complete message exceeds its transport limit.
    MessageTooLarge {
        /// Configured message byte limit.
        limit: usize,
        /// Actual message byte length.
        actual: usize,
    },
    /// One transport message violates its transport grammar.
    InvalidMessage(String),
    /// Underlying input or output failed.
    Io(std::io::Error),
    /// The transport is closed.
    Closed,
}

impl TransportError {
    /// Convert one transport receive failure, recognizing peer closure.
    pub(crate) fn from_receive(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::UnexpectedEof => Self::Closed,
            _ => Self::Io(error),
        }
    }
}

impl std::fmt::Display for TransportError {
    /// Format this transport failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "transport message exceeds {limit} bytes: {actual}"
                )
            }
            Self::InvalidMessage(message) => {
                write!(formatter, "invalid transport message: {message}")
            }
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::Closed => write!(formatter, "transport is closed"),
        }
    }
}

impl std::error::Error for TransportError {
    /// Return the underlying transport failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::MessageTooLarge { .. } | Self::InvalidMessage(_) | Self::Closed => None,
        }
    }
}

impl From<std::io::Error> for TransportError {
    /// Convert one input or output failure.
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Failure to accept a WebSocket RPC connection.
#[derive(Debug)]
pub enum WebSocketError {
    /// Underlying input or output failure.
    Io(std::io::Error),
    /// HTTP request parsing failure.
    Http(httparse::Error),
    /// Invalid WebSocket upgrade request.
    Handshake(&'static str),
    /// Unsupported WebSocket message.
    UnsupportedMessage(&'static str),
    /// WebSocket message exceeds the configured limit.
    MessageTooLarge {
        /// Configured byte limit.
        limit: usize,
        /// Actual byte length.
        actual: usize,
    },
}

impl std::fmt::Display for WebSocketError {
    /// Format this WebSocket failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "WebSocket I/O failed: {error}"),
            Self::Http(error) => write!(formatter, "WebSocket HTTP parsing failed: {error}"),
            Self::Handshake(message) => write!(formatter, "WebSocket handshake failed: {message}"),
            Self::UnsupportedMessage(message) => {
                write!(formatter, "unsupported WebSocket message: {message}")
            }
            Self::MessageTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "WebSocket message exceeds {limit} bytes: {actual}"
                )
            }
        }
    }
}

impl std::error::Error for WebSocketError {
    /// Return the underlying failure when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WebSocketError {
    /// Convert an input or output failure.
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<httparse::Error> for WebSocketError {
    /// Convert an HTTP parsing failure.
    fn from(error: httparse::Error) -> Self {
        Self::Http(error)
    }
}

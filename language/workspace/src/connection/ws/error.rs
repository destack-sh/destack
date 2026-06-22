use crate::protocol::FrameError;

/// Errors for workspace WebSocket connections.
#[derive(Debug)]
pub enum WorkspaceWebSocketError {
    /// Io error.
    Io(std::io::Error),
    /// HTTP request parse error.
    Http(httparse::Error),
    /// Invalid WebSocket handshake request.
    Handshake(&'static str),
    /// Frame codec error.
    Frame(FrameError),
    /// WebSocket connection closed.
    Closed,
    /// Unsupported WebSocket frame.
    UnsupportedFrame(&'static str),
    /// WebSocket payload is too large.
    PayloadTooLarge { limit: usize, actual: usize },
}

impl std::fmt::Display for WorkspaceWebSocketError {
    /// Format the WebSocket error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "workspace websocket io error: {error}"),
            Self::Http(error) => write!(formatter, "workspace websocket http error: {error}"),
            Self::Handshake(message) => {
                write!(formatter, "workspace websocket handshake error: {message}")
            }
            Self::Frame(error) => write!(formatter, "{error}"),
            Self::Closed => write!(formatter, "workspace websocket closed"),
            Self::UnsupportedFrame(message) => {
                write!(
                    formatter,
                    "unsupported workspace websocket frame: {message}"
                )
            }
            Self::PayloadTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "workspace websocket payload too large, limit {limit}, actual {actual}"
                )
            }
        }
    }
}

impl std::error::Error for WorkspaceWebSocketError {}

impl From<std::io::Error> for WorkspaceWebSocketError {
    /// Convert an io error into a WebSocket error.
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<httparse::Error> for WorkspaceWebSocketError {
    /// Convert an HTTP parse error into a WebSocket error.
    fn from(error: httparse::Error) -> Self {
        Self::Http(error)
    }
}

impl From<FrameError> for WorkspaceWebSocketError {
    /// Convert a protocol frame error into a WebSocket error.
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

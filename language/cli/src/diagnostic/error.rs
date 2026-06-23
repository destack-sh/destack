use std::error::Error;
use std::fmt;

/// Result type for console operations.
pub type ConsoleResult<T> = Result<T, ConsoleError>;

/// Error type for console operations.
#[derive(Debug)]
pub enum ConsoleError {
    /// Simple message error.
    Message(String),
    /// Wrapped error from lower layers.
    Other(Box<dyn Error + Send + Sync>),
}

impl ConsoleError {
    /// Build a message error.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl fmt::Display for ConsoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsoleError::Message(message) => write!(formatter, "{message}"),
            ConsoleError::Other(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ConsoleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConsoleError::Message(_) => None,
            ConsoleError::Other(error) => Some(error.as_ref()),
        }
    }
}

impl From<String> for ConsoleError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for ConsoleError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}

impl From<Box<dyn Error + Send + Sync>> for ConsoleError {
    fn from(error: Box<dyn Error + Send + Sync>) -> Self {
        Self::Other(error)
    }
}

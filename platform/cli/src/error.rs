use std::error::Error;
use std::fmt;

/// Result type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;

/// Error type for CLI operations.
#[derive(Debug)]
pub enum CliError {
    /// Simple message error.
    Message(String),
    /// Wrapped error from lower layers.
    Other(Box<dyn Error + Send + Sync>),
}

impl CliError {
    /// Build a message error.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Message(message) => write!(formatter, "{message}"),
            CliError::Other(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CliError::Message(_) => None,
            CliError::Other(error) => Some(error.as_ref()),
        }
    }
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for CliError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}

impl From<Box<dyn Error + Send + Sync>> for CliError {
    fn from(error: Box<dyn Error + Send + Sync>) -> Self {
        Self::Other(error)
    }
}

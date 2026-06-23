use std::error;
use std::fmt::{self, Display, Formatter};

/// Result returned by the Rust bridge facade.
pub type Result<T> = std::result::Result<T, Error>;

/// Error returned by the Rust bridge facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Error message.
    message: String,
}

impl Error {
    /// Create one bridge error from a displayable error.
    pub fn new(error: impl ToString) -> Self {
        Self {
            message: error.to_string(),
        }
    }

    /// Return this error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl error::Error for Error {}

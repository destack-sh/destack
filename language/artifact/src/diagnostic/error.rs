use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Error produced while finalizing provider diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    /// A provider diagnostic could not produce a valid source anchor.
    InvalidAnchor {
        /// The error message.
        message: String,
    },
}

impl Display for DiagnosticError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAnchor { message } => {
                write!(formatter, "invalid diagnostic anchor: {message}")
            }
        }
    }
}

impl Error for DiagnosticError {}

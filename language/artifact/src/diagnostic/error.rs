use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::DiagnosticAnchor;

/// Error produced while finalizing provider diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    /// A provider diagnostic could not produce a valid source site.
    InvalidSite {
        /// The error message.
        message: String,
    },
    /// A diagnostic anchor does not match a resolvable source artifact.
    InvalidAnchor {
        /// The invalid anchor.
        anchor: DiagnosticAnchor,
        /// The error message.
        message: String,
    },
}

impl Display for DiagnosticError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSite { message } => {
                write!(formatter, "invalid diagnostic site: {message}")
            }
            Self::InvalidAnchor { anchor, message } => {
                write!(formatter, "invalid diagnostic anchor {anchor:?}: {message}")
            }
        }
    }
}

impl Error for DiagnosticError {}

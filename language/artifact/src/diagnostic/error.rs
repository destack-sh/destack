use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::DiagnosticAnchor;

/// Error produced while finalizing provider diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    /// A provider diagnostic could not produce a valid source anchor.
    InvalidProviderAnchor {
        /// The error message.
        message: String,
    },
    /// The primary diagnostic anchor cannot be resolved.
    UnresolvedPrimary {
        /// The unresolved anchor.
        anchor: DiagnosticAnchor,
    },
    /// A secondary diagnostic label anchor cannot be resolved.
    UnresolvedLabel {
        /// The unresolved anchor.
        anchor: DiagnosticAnchor,
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
            Self::InvalidProviderAnchor { message } => {
                write!(formatter, "invalid provider diagnostic anchor: {message}")
            }
            Self::UnresolvedPrimary { anchor } => {
                write!(
                    formatter,
                    "diagnostic primary anchor cannot be resolved: {anchor:?}"
                )
            }
            Self::UnresolvedLabel { anchor } => {
                write!(
                    formatter,
                    "diagnostic label anchor cannot be resolved: {anchor:?}"
                )
            }
            Self::InvalidAnchor { anchor, message } => {
                write!(formatter, "invalid diagnostic anchor {anchor:?}: {message}")
            }
        }
    }
}

impl Error for DiagnosticError {}

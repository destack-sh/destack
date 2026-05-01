use destack_core::Color;
use serde::{Deserialize, Serialize};

/// The level of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Note (informative message).
    Note = 1,
    /// Warning (non-critical issue).
    Warning = 2,
    /// Error (critical issue).
    Error = 3,
}

impl DiagnosticSeverity {
    /// Get the family name of the severity.
    pub fn family_name(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Get the color of the severity.
    pub fn color(&self) -> Color {
        match self {
            Self::Note => Color::BrightBlue,
            Self::Warning => Color::BrightYellow,
            Self::Error => Color::BrightRed,
        }
    }
}

/// Extra semantic tag for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticTag {
    /// The diagnostic marks unused or unnecessary source.
    Unnecessary,
    /// The diagnostic marks deprecated source.
    Deprecated,
}

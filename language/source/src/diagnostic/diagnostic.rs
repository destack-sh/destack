use crate::{Color, FileId, LabeledSpan, Suggestion};

/// The level of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Get the stage letter of the severity.
    pub fn stage_letter(&self) -> &'static str {
        match self {
            Self::Note => "N",
            Self::Warning => "W",
            Self::Error => "E",
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

/// A Diagnostic.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Diagnostic {
    /// The stable identifier of the diagnostic (like `E001` or `W017`).
    pub code: String,
    /// The DiagnosticSeverity of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// The message of the diagnostic.
    pub message: String,
    /// The primary source of the diagnostic.
    pub file_id: FileId,
    /// The primary span of the diagnostic.
    pub primary_span: LabeledSpan,
    /// The secondary spans of the diagnostic.
    pub secondary_spans: Option<Vec<LabeledSpan>>,
    /// The suggestions for the diagnostic.
    pub suggestions: Option<Vec<Suggestion>>,
}

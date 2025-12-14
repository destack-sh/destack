use destack_base::Color;

use crate::{FileId, LabeledSpan, Suggestion};

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
    /// The original code of the diagnostic (if changed by options).
    pub original_code: Option<String>,
    /// The DiagnosticSeverity of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// The original severity of the diagnostic (if changed by options).
    pub original_severity: Option<DiagnosticSeverity>,
    /// The message of the diagnostic.
    pub message: String,
    /// The primary source of the diagnostic.
    pub file_id: FileId,
    /// The primary span of the diagnostic.
    pub primary_span: LabeledSpan,
    /// The spans to highlight within the primary span.
    pub primary_highlight_spans: Option<Vec<LabeledSpan>>,
    /// The secondary spans of the diagnostic.
    pub secondary_spans: Option<Vec<LabeledSpan>>,
    /// The suggestions for the diagnostic.
    pub suggestions: Option<Vec<Suggestion>>,
}

/// Diagnostic options for re-mapping.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticOptions {
    /// Which warning codes to error on (as errors).
    pub error_warnings: Vec<String>,
    /// Which error codes to suppress (as warnings).
    pub suppress_errors: Vec<String>,
    /// Which warning codes to suppress.
    pub suppress_warnings: Vec<String>,
}

impl DiagnosticOptions {
    /// Map a diagnostic to its adjusted diagnostic.
    pub fn map(&self, mut diagnostic: Diagnostic) -> Option<Diagnostic> {
        // retain original code/severity
        diagnostic.original_code = Some(diagnostic.code.clone());
        diagnostic.original_severity = Some(diagnostic.severity);

        // map severity
        // error -> warning
        if diagnostic.severity == DiagnosticSeverity::Error
            && self.suppress_errors.contains(&diagnostic.code)
        {
            diagnostic.severity = DiagnosticSeverity::Warning;
        }
        // warning -> error
        else if diagnostic.severity == DiagnosticSeverity::Warning
            && self.error_warnings.contains(&diagnostic.code)
        {
            diagnostic.severity = DiagnosticSeverity::Error;
        }
        // warning -> none
        else if diagnostic.severity == DiagnosticSeverity::Warning
            && self.suppress_warnings.contains(&diagnostic.code)
        {
            return None;
        }
        Some(diagnostic)
    }

    /// Map a sequence of diagnostics to their adjusted diagnostics.
    pub fn map_all(&self, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .filter_map(|d| self.map(d.clone()))
            .collect()
    }
}

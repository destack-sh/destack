use crate::Suggestion;
use dyst_language_source::{LabeledSpan, SourceId};

/// The kind of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum DiagnosticKind {
    /// Parse error (invalid syntax).
    Parse,
    // type, static, ..
}

/// The level of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum Severity {
    /// Error (critical issue).
    Error,
    /// Warning (non-critical issue).
    Warning,
    /// Note (informative message).
    Note,
    /// Help (suggestive message).
    Help,
}

/// A Diagnostic.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Diagnostic {
    /// The diagnostic ID of the diagnostic (like `E001` or `W017`).
    pub id: String,
    /// The kind of the diagnostic.
    pub kind: DiagnosticKind,
    /// The severity of the diagnostic.
    pub severity: Severity,
    /// The message of the diagnostic.
    pub message: String,
    /// The primary source of the diagnostic.
    pub source: SourceId,
    /// The primary span of the diagnostic.
    pub primary_span: LabeledSpan,
    /// The secondary spans of the diagnostic.
    pub secondary_spans: Option<Vec<LabeledSpan>>,
    /// The suggestions for the diagnostic.
    pub suggestions: Option<Vec<Suggestion>>,
}

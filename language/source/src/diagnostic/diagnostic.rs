use crate::{FileId, LabeledSpan, Suggestion};

/// The kind of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
pub enum DiagnosticKind {
    /// Parse error (invalid syntax).
    Parse,
    // type, static, ..
}

/// The level of a diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
pub enum DiagnosticSeverity {
    /// Note (informative message).
    Note = 1,
    /// Warning (non-critical issue).
    Warning = 2,
    /// Error (critical issue).
    Error = 3,
}

/// A Diagnostic.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Diagnostic {
    /// The kind of the diagnostic.
    pub kind: DiagnosticKind,
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

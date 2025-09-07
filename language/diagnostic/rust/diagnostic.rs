use crate::{LabeledSpan, Suggestion};

/// The kind of a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    /// Parse error (invalid syntax).
    Parse,
    /// Type error (invalid type).
    Type,
    /// Static error (static evaluation error).
    Static,
}

/// The level of a diagnostic.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The unique ID of the diagnostic.
    pub id: i32,

    /// The severity of the diagnostic.
    pub severity: Severity,

    /// The message of the diagnostic.
    pub message: String,

    /// The primary span of the diagnostic.
    pub primary_span: Option<LabeledSpan>,

    /// The secondary spans of the diagnostic.
    pub secondary_spans: Option<Vec<LabeledSpan>>,

    /// The suggestions for the diagnostic.
    pub suggestions: Option<Vec<Suggestion>>,
}

impl Diagnostic {
    // nocheckin: render diagnostics
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{:?}: {}", self.severity, self.message));
        out
    }
}

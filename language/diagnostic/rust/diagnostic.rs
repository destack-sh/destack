use dyst_language_source::Span;

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
pub enum DiagnosticLevel {
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
pub trait Diagnostic {
    /// Get the unique ID of the diagnostic.
    fn id(&self) -> i32;

    /// Get the level of the diagnostic.
    fn level(&self) -> DiagnosticLevel;

    /// Get the message of the diagnostic.
    fn message(&self) -> String;

    /// Get the primary span of the diagnostic.
    fn primary_span(&self) -> Option<Span>;

    /// Get the secondary spans of the diagnostic.
    fn secondary_spans(&self) -> Option<&[Span]>;
}

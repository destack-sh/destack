use dyst_language_diagnostic::Diagnostic;

/// A session for diagnostic operations.
#[derive(Debug)]
pub struct Session {
    /// The diagnostics emitted in this session.
    pub diagnostics: Vec<Diagnostic>,
}

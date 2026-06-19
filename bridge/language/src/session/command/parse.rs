use crate::{Diagnostic, DirParsed, bridge};

/// One parser output.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    /// Parsed DIR artifact projection.
    pub parsed: DirParsed,
    /// Diagnostics emitted by parsing.
    pub diagnostics: Vec<Diagnostic>,
}

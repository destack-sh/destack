use dyst_source::{Diagnostic, File, Severity};
use tower_lsp_server::lsp_types as lsp;

use crate::source::byte_span_to_range;

/// Convert a Dyst diagnostic to an LSP diagnostic.
pub fn diagnostic_to_lsp_diagnostic(diagnostic: &Diagnostic, source: &File) -> lsp::Diagnostic {
    // span
    let range = byte_span_to_range(source, diagnostic.primary_span.span);

    // severity
    let severity = match diagnostic.severity {
        Severity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        Severity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        Severity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
        Severity::Help => Some(lsp::DiagnosticSeverity::HINT),
    };

    // diagnostic
    lsp::Diagnostic {
        range,
        severity,
        code: None,
        code_description: None,
        source: Some("destack".to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

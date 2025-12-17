use destack_source::{Diagnostic, DiagnosticSeverity, File};
use tower_lsp_server::lsp_types as lsp;

use super::common::byte_span_to_range;

/// Convert a Destack diagnostic to an LSP diagnostic.
pub fn diagnostic_to_lsp_diagnostic(diagnostic: &Diagnostic, source: &File) -> lsp::Diagnostic {
    // span
    let range = byte_span_to_range(source, diagnostic.primary_span.span);

    // severity
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        DiagnosticSeverity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        DiagnosticSeverity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
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

use dyst_language_diagnostic::{Diagnostic, Severity};
use dyst_language_source::Source;
use tower_lsp_server::lsp_types as lsp;

use crate::source::byte_to_utf16_position;

/// Convert a Dyst diagnostic to an LSP diagnostic.
pub fn diagnostic_to_lsp_diagnostic(diagnostic: &Diagnostic, source: &Source) -> lsp::Diagnostic {
    // convert byte span to LSP range
    let start_pos = byte_to_utf16_position(source, diagnostic.primary_span.span.start)
        .map(|(line, character)| lsp::Position { line, character })
        .unwrap_or_default();

    let end_pos = byte_to_utf16_position(source, diagnostic.primary_span.span.end)
        .map(|(line, character)| lsp::Position { line, character })
        .unwrap_or_default();

    let range = lsp::Range {
        start: start_pos,
        end: end_pos,
    };

    // map diagnostic severity
    let severity = match diagnostic.severity {
        Severity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        Severity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        Severity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
        Severity::Help => Some(lsp::DiagnosticSeverity::HINT),
    };

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

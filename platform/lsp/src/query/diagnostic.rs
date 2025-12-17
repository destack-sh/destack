use destack_source::{Diagnostic, DiagnosticSeverity, File};
use destack_workspace::{Session, query};
use tower_lsp_server::lsp_types as lsp;

use super::common::byte_span_to_range;
use super::refactor::batch_edit_to_workspace_edit;

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

    // related information from secondary spans
    let related_information = diagnostic.secondary_spans.as_ref().map(|spans| {
        spans
            .iter()
            .filter_map(|labeled| {
                let uri = source.uri.as_ref().parse::<lsp::Uri>().ok()?;
                Some(lsp::DiagnosticRelatedInformation {
                    location: lsp::Location {
                        uri,
                        range: byte_span_to_range(source, labeled.span),
                    },
                    message: labeled.label.clone(),
                })
            })
            .collect()
    });

    // diagnostic
    lsp::Diagnostic {
        range,
        severity,
        code: Some(lsp::NumberOrString::String(diagnostic.code.clone())),
        code_description: None,
        source: Some("destack".to_string()),
        message: diagnostic.message.clone(),
        related_information,
        tags: None,
        data: None,
    }
}

/// Convert a code action to an LSP code action.
pub fn code_action_to_lsp(
    session: &Session,
    action: &query::CodeAction,
) -> Option<lsp::CodeActionOrCommand> {
    let kind = match action.kind {
        query::CodeActionKind::QuickFix => lsp::CodeActionKind::QUICKFIX,
        query::CodeActionKind::Refactor => lsp::CodeActionKind::REFACTOR,
        query::CodeActionKind::RefactorExtract => lsp::CodeActionKind::REFACTOR_EXTRACT,
        query::CodeActionKind::RefactorInline => lsp::CodeActionKind::REFACTOR_INLINE,
        query::CodeActionKind::RefactorRewrite => lsp::CodeActionKind::REFACTOR_REWRITE,
        query::CodeActionKind::Source => lsp::CodeActionKind::SOURCE,
        query::CodeActionKind::SourceOrganizeImports => {
            lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS
        }
        query::CodeActionKind::SourceFixAll => lsp::CodeActionKind::SOURCE_FIX_ALL,
    };

    let edit = if action.edits.is_empty() {
        None
    } else {
        Some(batch_edit_to_workspace_edit(session, &action.edits))
    };

    let disabled = action
        .disabled_reason
        .as_ref()
        .map(|reason| lsp::CodeActionDisabled {
            reason: reason.clone(),
        });

    Some(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: action.title.clone(),
        kind: Some(kind),
        diagnostics: None,
        edit,
        command: None,
        is_preferred: Some(action.is_preferred),
        disabled,
        data: None,
    }))
}

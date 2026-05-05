use std::sync::Arc;

use destack_source::{
    Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticTag, File, FileId,
};
use destack_workspace::{Repository, Revision};
use serde_json::Value;
use {destack_lsp_types as lsp, destack_query as query};

use super::common::byte_span_to_range;
use super::refactor::batch_edit_to_workspace_edit;

/// Convert a Destack diagnostic to an LSP diagnostic.
pub fn diagnostic_to_lsp_diagnostic<F>(
    diagnostic: &Diagnostic,
    file_for_id: &F,
) -> Option<lsp::Diagnostic>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let primary = diagnostic.primary_label();
    let primary_file = file_for_id(primary.span.file)?;
    if !label_matches_file(primary, &primary_file) {
        return None;
    }

    let primary_span = primary.span;
    let range = byte_span_to_range(&primary_file, primary_span);

    // severity
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        DiagnosticSeverity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        DiagnosticSeverity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
    };

    // related locations
    let related_locations = diagnostic
        .labels()
        .filter_map(|label| {
            let file = file_for_id(label.span.file)?;
            if !label_matches_file(label, &file) {
                return None;
            }

            let uri = file.uri.as_ref().parse::<lsp::Uri>().ok()?;

            Some(lsp::DiagnosticRelatedInformation {
                location: lsp::Location {
                    uri,
                    range: byte_span_to_range(&file, label.span),
                },
                message: label
                    .message
                    .clone()
                    .unwrap_or_else(|| "related location".to_string()),
            })
        })
        .collect::<Vec<_>>();
    let related_information = if related_locations.is_empty() {
        None
    } else {
        Some(related_locations)
    };

    // tags
    let tags = if diagnostic.tags.is_empty() {
        None
    } else {
        Some(
            diagnostic
                .tags
                .iter()
                .map(|tag| match tag {
                    DiagnosticTag::Unnecessary => lsp::DiagnosticTag::UNNECESSARY,
                    DiagnosticTag::Deprecated => lsp::DiagnosticTag::DEPRECATED,
                })
                .collect(),
        )
    };

    // diagnostic
    Some(lsp::Diagnostic {
        range,
        severity,
        code: Some(lsp::NumberOrString::String(diagnostic.code.clone())),
        code_description: None,
        source: Some("destack".to_string()),
        message: diagnostic.message.clone(),
        related_information,
        tags,
        data: None,
    })
}

/// Return true when one diagnostic label still points at the same file content.
fn label_matches_file(label: &DiagnosticLabel, file: &File) -> bool {
    label.span.file == file.id && label.content == file.content_id()
}

/// Convert a workspace code action kind to an LSP code action kind.
pub fn code_action_kind_to_lsp(kind: query::CodeActionKind) -> lsp::CodeActionKind {
    match kind {
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
    }
}

/// Convert a code action to an LSP code action.
pub fn code_action_to_lsp(
    repository: &Repository,
    revision: Revision,
    action: &query::CodeAction,
    include_edit: bool,
    data: Option<Value>,
) -> Option<lsp::CodeActionOrCommand> {
    let edit = if include_edit && !action.edits.is_empty() {
        Some(batch_edit_to_workspace_edit(
            repository,
            revision,
            &action.edits,
        ))
    } else {
        None
    };

    let disabled = action
        .disabled_reason
        .as_ref()
        .map(|reason| lsp::CodeActionDisabled {
            reason: reason.clone(),
        });

    Some(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: action.title.clone(),
        kind: Some(code_action_kind_to_lsp(action.kind)),
        diagnostics: None,
        edit,
        command: None,
        is_preferred: Some(action.is_preferred),
        disabled,
        data,
    }))
}

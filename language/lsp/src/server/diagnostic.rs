use std::hash::Hash;

use destack_core::StableHasher;
use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::{Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticTag, File, Span};
use serde_json::Value;

use super::error::{internal_error, workspace_error};
use super::source::SourceFiles;
use super::{edit, position};
use crate::uri;

/// Convert a Destack diagnostic to an LSP diagnostic.
pub(super) fn item(
    diagnostic: &Diagnostic,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::Diagnostic> {
    let primary = diagnostic.primary_label();
    let primary_file = files.file(primary.target.file()).map_err(workspace_error)?;
    if !label_matches_file(primary, &primary_file) {
        return Err(internal_error(format!(
            "diagnostic {} primary label does not match source file {:?}",
            diagnostic.id, primary_file.id
        )));
    }

    let primary_span = primary
        .target
        .span()
        .unwrap_or_else(|| Span::empty(primary.target.file()));
    let range = position::range(&primary_file, primary_span)?;

    // severity
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        DiagnosticSeverity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        DiagnosticSeverity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
    };

    // related locations
    let mut related_locations = Vec::with_capacity(diagnostic.labels.len());
    for label in diagnostic.labels() {
        let file = files.file(label.target.file()).map_err(workspace_error)?;
        if !label_matches_file(label, &file) {
            return Err(internal_error(format!(
                "diagnostic {} label does not match source file {:?}",
                diagnostic.id, file.id
            )));
        }

        let span = label
            .target
            .span()
            .unwrap_or_else(|| Span::empty(label.target.file()));
        let uri = uri::file(&file)?;
        let range = position::range(&file, span)?;
        let message = label
            .message
            .clone()
            .unwrap_or_else(|| diagnostic.message.clone());

        related_locations.push(lsp::DiagnosticRelatedInformation {
            location: lsp::Location { uri, range },
            message,
        });
    }
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
    Ok(lsp::Diagnostic {
        range,
        severity,
        code: Some(lsp::NumberOrString::String(diagnostic.id.clone())),
        code_description: None,
        source: Some("destack".to_string()),
        message: diagnostic.message.clone(),
        related_information,
        tags,
        data: None,
    })
}

/// Compute a deterministic result id for a diagnostics payload.
pub(super) fn result_id(diagnostics: &[Diagnostic]) -> String {
    let mut hasher = StableHasher::new();
    diagnostics.hash(&mut hasher);

    format!("{:x}", hasher.finish_u64())
}

/// Return true when one diagnostic label still points at the same file content.
fn label_matches_file(label: &DiagnosticLabel, file: &File) -> bool {
    label.target.file() == file.id && label.content == file.content_id()
}

/// Convert a workspace code action kind to an LSP code action kind.
fn code_action_kind(kind: query::CodeActionKind) -> lsp::CodeActionKind {
    match kind {
        query::CodeActionKind::QuickFix => lsp::CodeActionKind::QUICKFIX,
        query::CodeActionKind::RefactorExtract => lsp::CodeActionKind::REFACTOR_EXTRACT,
        query::CodeActionKind::RefactorInline => lsp::CodeActionKind::REFACTOR_INLINE,
    }
}

/// Convert a code action to an LSP code action.
pub(super) fn code_action(
    action: &query::CodeAction,
    include_edit: bool,
    data: Option<Value>,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::CodeActionOrCommand> {
    let edit = if include_edit && !action.patches.is_empty() {
        Some(edit::workspace(&action.patches, files)?)
    } else {
        None
    };

    Ok(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: action.title.clone(),
        kind: Some(code_action_kind(action.kind)),
        diagnostics: None,
        edit,
        command: None,
        is_preferred: Some(action.is_preferred),
        disabled: None,
        data,
    }))
}

use std::path::Path;
use std::sync::Arc;

use destack_core::StableHasher;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::{
    Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticTag, File, FileId,
};
use destack_workspace::{
    DiagnosticSnapshot, DiagnosticsRequest, Error, ReloadReason, ReloadRequest, Workspace,
};
use serde_json::Value;
use std::hash::Hash;

use super::edit::patch_set_to_workspace_edit;
use super::position::byte_span_to_range;

/// Return diagnostic snapshots for one root.
pub(super) fn diagnostic_root_snapshots(
    workspace: &dyn Workspace,
    root: &Path,
) -> Result<Vec<DiagnosticSnapshot>, Error> {
    let revision = workspace.revision(root)?;
    let views = workspace.diagnostics(DiagnosticsRequest::Root(root.to_path_buf()))?;

    Ok(views
        .iter()
        .map(|view| DiagnosticSnapshot::new(revision, view))
        .collect())
}

/// Return diagnostic snapshots for the root owning one path.
pub(super) fn diagnostic_path_snapshots(
    workspace: &dyn Workspace,
    path: &Path,
) -> Result<Vec<DiagnosticSnapshot>, Error> {
    let root = workspace.root(path)?;

    diagnostic_root_snapshots(workspace, &root)
}

/// Return one diagnostic snapshot for a path.
pub(super) fn diagnostic_file_snapshot(
    workspace: &dyn Workspace,
    path: &Path,
) -> Result<Option<DiagnosticSnapshot>, Error> {
    let root = workspace.root(path)?;
    let revision = workspace.revision(&root)?;
    let views = workspace.diagnostics(DiagnosticsRequest::File(path.to_path_buf()))?;

    Ok(views
        .first()
        .map(|view| DiagnosticSnapshot::new(revision, view)))
}

/// Return diagnostic snapshots for every open root.
pub(super) fn diagnostic_snapshots(
    workspace: &dyn Workspace,
) -> Result<Vec<DiagnosticSnapshot>, Error> {
    let mut diagnostics = Vec::new();
    for root in workspace.roots() {
        diagnostics.extend(diagnostic_root_snapshots(workspace, &root)?);
    }

    Ok(diagnostics)
}

/// Reload all roots and return current diagnostic snapshots.
pub(super) fn reload_diagnostic_snapshots(
    workspace: &dyn Workspace,
) -> Result<Vec<DiagnosticSnapshot>, Error> {
    workspace.reload(ReloadRequest {
        roots: Vec::new(),
        reason: ReloadReason::Manual,
    })?;

    diagnostic_snapshots(workspace)
}

/// Convert a Destack diagnostic to an LSP diagnostic.
pub(super) fn diagnostic_to_lsp_diagnostic<F>(
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

/// Compute a deterministic result id for a diagnostics payload.
pub(super) fn diagnostic_result_id(diagnostics: &[Diagnostic]) -> String {
    let mut hasher = StableHasher::new();
    diagnostics.len().hash(&mut hasher);
    for diagnostic in diagnostics {
        diagnostic.code.hash(&mut hasher);
        diagnostic.message.hash(&mut hasher);
        let primary = diagnostic.primary_label();
        primary.content.hash(&mut hasher);
        let primary_span = primary.span;
        primary_span.start.hash(&mut hasher);
        primary_span.end.hash(&mut hasher);
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => 0u8,
            DiagnosticSeverity::Warning => 1u8,
            DiagnosticSeverity::Note => 2u8,
        };
        severity.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish_u64())
}

/// Return true when one diagnostic label still points at the same file content.
fn label_matches_file(label: &DiagnosticLabel, file: &File) -> bool {
    label.span.file == file.id && label.content == file.content_id()
}

/// Convert a workspace code action kind to an LSP code action kind.
pub(super) fn code_action_kind_to_lsp(kind: query::CodeActionKind) -> lsp::CodeActionKind {
    match kind {
        query::CodeActionKind::QuickFix => lsp::CodeActionKind::QUICKFIX,
        query::CodeActionKind::Refactor => lsp::CodeActionKind::REFACTOR,
        query::CodeActionKind::RefactorExtract => lsp::CodeActionKind::REFACTOR_EXTRACT,
        query::CodeActionKind::RefactorInline => lsp::CodeActionKind::REFACTOR_INLINE,
        query::CodeActionKind::RefactorRewrite => lsp::CodeActionKind::REFACTOR_REWRITE,
        query::CodeActionKind::Source => lsp::CodeActionKind::SOURCE,
        query::CodeActionKind::SourceFixAll => lsp::CodeActionKind::SOURCE_FIX_ALL,
    }
}

/// Convert a code action to an LSP code action.
pub(super) fn code_action_to_lsp(
    action: &query::CodeAction,
    include_edit: bool,
    data: Option<Value>,
    file_for_id: &mut impl FnMut(FileId) -> Option<Arc<File>>,
) -> Option<lsp::CodeActionOrCommand> {
    let edit = if include_edit && !action.patches.is_empty() {
        Some(patch_set_to_workspace_edit(&action.patches, file_for_id))
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

use destack_source::{Applicability, BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;

/// Kind of code action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic.
    QuickFix,
    /// Refactoring action.
    Refactor,
    /// Extract refactoring (extract function, variable, etc.).
    RefactorExtract,
    /// Inline refactoring.
    RefactorInline,
    /// Rewrite refactoring.
    RefactorRewrite,
    /// Source organization (imports, etc.).
    Source,
    /// Organize imports.
    SourceOrganizeImports,
    /// Fix all issues of a type.
    SourceFixAll,
}

/// A code action (quick fix or refactoring).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeAction {
    /// The title shown in the UI.
    pub title: String,
    /// The kind of action.
    pub kind: CodeActionKind,
    /// Edits to apply.
    pub edits: BatchEdit,
    /// Whether this is the preferred action for its diagnostics.
    pub is_preferred: bool,
    /// Whether this action is disabled (with reason).
    pub disabled_reason: Option<String>,
    /// The diagnostic code this action fixes (if from a diagnostic).
    pub diagnostic_code: Option<String>,
}

impl CodeAction {
    /// Create a quick fix.
    pub fn quick_fix(title: impl Into<String>, edits: BatchEdit) -> Self {
        Self {
            title: title.into(),
            kind: CodeActionKind::QuickFix,
            edits,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Create a refactoring.
    pub fn refactor(title: impl Into<String>, kind: CodeActionKind, edits: BatchEdit) -> Self {
        Self {
            title: title.into(),
            kind,
            edits,
            is_preferred: false,
            disabled_reason: None,
            diagnostic_code: None,
        }
    }

    /// Mark as preferred.
    pub fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.disabled_reason = Some(reason.into());
        self
    }

    /// Set the diagnostic code this action fixes.
    pub fn with_diagnostic_code(mut self, code: impl Into<String>) -> Self {
        self.diagnostic_code = Some(code.into());
        self
    }
}

/// Context for code action requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CodeActionContext {
    /// Requested action kinds (empty = all).
    pub only: Vec<CodeActionKind>,
    /// Whether to include disabled actions.
    pub include_disabled: bool,
}

/// Request code actions for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeActionsRequest {
    /// The document URI.
    pub uri: Uri,
    /// The start byte offset in the document.
    pub start: u32,
    /// The end byte offset in the document.
    pub end: u32,
    /// The code action context.
    pub context: CodeActionContext,
}

/// Response payload for code actions queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeActionsResponse {
    /// Code actions.
    pub actions: Vec<CodeAction>,
}

/// Get code actions for a range in a file.
///
/// Includes quick fixes from diagnostics and available refactorings.
pub fn code_actions(
    session: &Session,
    file: FileId,
    range: Span,
    context: &CodeActionContext,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // collect quick fixes from diagnostics
    collect_diagnostic_fixes(session, file, range, &mut actions);

    // NOTE #Incomplete: auto import actions need structured diagnostic data
    // message parsing is not robust enough for a gold standard experience

    // filter by context.only if specified
    if !context.only.is_empty() {
        actions.retain(|a| context.only.contains(&a.kind));
    }

    // filter out disabled unless requested
    if !context.include_disabled {
        actions.retain(|a| a.disabled_reason.is_none());
    }

    // sort deterministically by kind, preference, title, and edit shape
    actions.sort_by_cached_key(code_action_key);

    // deduplicate identical actions after sorting
    actions.dedup_by(|left, right| code_action_key(left) == code_action_key(right));

    actions
}

/// Collect quick fixes from diagnostics that overlap with the range.
fn collect_diagnostic_fixes(
    session: &Session,
    file: FileId,
    range: Span,
    actions: &mut Vec<CodeAction>,
) {
    // iterate through all programs to find diagnostics for this file
    for program in session.programs.iter() {
        let program = program.value();
        let diagnostics = program.diagnostic_store.diagnostics_for_file(file);
        for diagnostic in diagnostics {
            // skip diagnostics for other files
            if diagnostic.file_id != file {
                continue;
            }

            // check if diagnostic overlaps with the requested range
            let diag_span = &diagnostic.primary_span.span;
            if diag_span.end < range.start || diag_span.start > range.end {
                continue;
            }

            // convert suggestions to code actions
            if let Some(suggestions) = &diagnostic.suggestions {
                for suggestion in suggestions {
                    // skip non-automatic suggestions
                    if suggestion.applicability != Applicability::Automatic {
                        continue;
                    }

                    // create edit from suggestion
                    let Some(replacement) = &suggestion.replacement else {
                        continue;
                    };

                    // build the batch edit from suggestion spans
                    let mut file_edit = FileEdit::new(file);
                    for labeled_span in &suggestion.spans {
                        file_edit.push(Edit::replace(labeled_span.span, replacement.clone()));
                    }

                    if file_edit.is_empty() {
                        continue;
                    }

                    let mut batch_edit = BatchEdit::new();
                    batch_edit.files.push(file_edit);

                    let action = CodeAction::quick_fix(&suggestion.message, batch_edit)
                        .with_diagnostic_code(&diagnostic.code)
                        .preferred();

                    actions.push(action);
                }
            }
        }
    }
}

/// Build a stable ordering key for a code action.
fn code_action_key(action: &CodeAction) -> (u8, u8, String, String, String) {
    // rank kinds so quick fixes come before refactors and source actions
    let kind_rank = code_action_kind_rank(action.kind);

    // prefer preferred actions within the same kind
    let preferred_rank = if action.is_preferred { 0 } else { 1 };

    // include the diagnostic code when present to keep related fixes grouped
    let diagnostic_code = action.diagnostic_code.clone().unwrap_or_default();

    // fold the edit shape into the key to make ordering and deduplication stable
    let edit_key = batch_edit_key(&action.edits);

    (
        kind_rank,
        preferred_rank,
        action.title.clone(),
        diagnostic_code,
        edit_key,
    )
}

/// Rank code action kinds for stable ordering.
fn code_action_kind_rank(kind: CodeActionKind) -> u8 {
    match kind {
        CodeActionKind::QuickFix => 0,
        CodeActionKind::Refactor => 1,
        CodeActionKind::RefactorExtract => 2,
        CodeActionKind::RefactorInline => 3,
        CodeActionKind::RefactorRewrite => 4,
        CodeActionKind::Source => 5,
        CodeActionKind::SourceOrganizeImports => 6,
        CodeActionKind::SourceFixAll => 7,
    }
}

/// Build a stable key for a batch edit.
fn batch_edit_key(edit: &BatchEdit) -> String {
    // clone and sort file edits by file id
    let mut files = edit.files.clone();
    files.sort_by_key(|file_edit| file_edit.file.0);

    // serialize edits in a deterministic order
    let mut parts = Vec::new();
    for file_edit in files {
        let file_key = file_edit_key(&file_edit);
        parts.push(file_key);
    }

    parts.join("|")
}

/// Build a stable key for a file edit.
fn file_edit_key(file_edit: &FileEdit) -> String {
    // clone and sort edits by span and text
    let mut edits = file_edit.edits.clone();
    edits.sort_by_key(edit_key);

    // serialize all edits for this file
    let mut parts = Vec::new();
    parts.push(format!("file={}", file_edit.file.0));
    for edit in edits {
        let (start, end, span_file, text) = edit_key(&edit);
        parts.push(format!("{span_file}:{start}-{end}=>{text}"));
    }

    parts.join(",")
}

/// Build a stable key for a single edit.
fn edit_key(edit: &Edit) -> (u32, u32, u32, String) {
    // extract span coordinates and replacement text
    let span_file = edit.span.file.0;
    let start = edit.span.start;
    let end = edit.span.end;
    let text = edit.new_text.clone();

    (start, end, span_file, text)
}

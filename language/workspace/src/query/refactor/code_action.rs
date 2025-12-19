use destack_source::{Applicability, BatchEdit, Edit, FileEdit, FileId, Span};

use crate::Session;

/// Kind of code action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
pub struct CodeActionContext {
    /// Requested action kinds (empty = all).
    pub only: Vec<CodeActionKind>,
    /// Whether to include disabled actions.
    pub include_disabled: bool,
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

    // 1. collect quick fixes from diagnostics
    collect_diagnostic_fixes(session, file, range, &mut actions);

    // 2. filter by context.only if specified
    if !context.only.is_empty() {
        actions.retain(|a| context.only.contains(&a.kind));
    }

    // 3. filter out disabled unless requested
    if !context.include_disabled {
        actions.retain(|a| a.disabled_reason.is_none());
    }

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
        let diagnostics = program.diagnostics.iter();
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

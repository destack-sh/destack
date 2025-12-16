use destack_source::{BatchEdit, FileId, Span};

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
}

/// Context for code action requests.
#[derive(Debug, Clone, Default)]
pub struct CodeActionContext {
    /// Requested action kinds (empty = all).
    pub only: Vec<CodeActionKind>,
    /// Whether to include disabled actions.
    pub include_disabled: bool = false,
}

/// Get code actions for a range in a file.
///
/// Includes quick fixes from diagnostics and available refactorings.
pub fn code_actions(
    _session: &Session,
    _file: FileId,
    _range: Span,
    _context: &CodeActionContext,
) -> Vec<CodeAction> {
    // 1. get diagnostics that overlap with range
    // 2. collect quick fixes from those diagnostics (from linter)
    // 3. check for available refactorings at this location:
    //    - extract variable (if expression selected)
    //    - extract function (if statements selected)
    //    - inline variable (if on variable reference)
    //    - organize imports (if in imports section)
    // 4. filter by context.only if specified
    todo!("#Incomplete: code_actions")
}

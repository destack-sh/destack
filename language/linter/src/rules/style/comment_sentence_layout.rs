use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require each prose sentence to begin on its own source line.
    pub COMMENT_SENTENCE_LAYOUT {
        id: "comment-sentence-layout",
        summary: "Require each prose sentence to begin on its own source line",
        explanation: "Documentation and prose comments begin at most one sentence on each source line. Keeping sentence boundaries aligned with source lines makes edits, reviews, and generated documentation diffs local and predictable.",
        example: {
            reported: r#"
/// Open a session. The caller owns the returned handle.
function openSession(): Session;
"#,
            accepted: r#"
/// Open a session.
/// The caller owns the returned handle.
function openSession(): Session;
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check comment-sentence-layout.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

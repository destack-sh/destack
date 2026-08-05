use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require canonical punctuation for comments and documentation.
    pub COMMENT_PUNCTUATION {
        id: "comment-punctuation",
        summary: "Require canonical punctuation for comments and documentation",
        explanation: "Documentation sentences end with punctuation. Short inline comments are labels or action phrases and omit a final period; inline comments that contain complete sentences retain their punctuation.",
        example: {
            reported: r#"
/// Return the active session
function session(): Session;

// build the session index.
const sessions = indexSessions();
"#,
            accepted: r#"
/// Return the active session.
function session(): Session;

// build the session index
const sessions = indexSessions();
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check comment-punctuation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

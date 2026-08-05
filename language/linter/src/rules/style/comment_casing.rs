use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require canonical casing for comments and documentation.
    pub COMMENT_CASING {
        id: "comment-casing",
        summary: "Require canonical casing for comments and documentation",
        explanation: "Documentation is prose and begins with an uppercase letter. Inline comments organize code and begin with a lowercase action or short label. Acronyms, identifiers, code fragments, and recognized annotation keywords remain unchanged.",
        example: {
            reported: r#"
/// returns the active session.
function session(): Session;

// Build the session index
const sessions = indexSessions();
"#,
            accepted: r#"
/// Returns the active session.
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

/// Check comment-casing.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

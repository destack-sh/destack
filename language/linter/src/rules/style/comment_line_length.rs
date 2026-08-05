use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Limit authored comment and documentation lines to 100 columns.
    pub COMMENT_LINE_LENGTH {
        id: "comment-line-length",
        summary: "Limit authored comment and documentation lines to 100 columns",
        explanation: "Comment prose fits within the standard 100-column source width so it remains readable beside code and in diagnostics. Unbreakable URLs, code fragments, generated comments, and legal notices are exempt.",
        example: {
            reported: r#"
/// Return the authenticated session after validating every configured policy for the current incoming request.
function authenticate(request: &readonly Request): Result<Session, AuthenticationError>;
"#,
            accepted: r#"
/// Return the authenticated session after validating every configured policy.
///
/// The policies are selected from the current incoming request.
function authenticate(request: &readonly Request): Result<Session, AuthenticationError>;
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check comment-line-length.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

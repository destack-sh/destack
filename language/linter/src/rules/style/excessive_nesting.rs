use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow control flow nested beyond the canonical depth.
    pub EXCESSIVE_NESTING {
        id: "excessive-nesting",
        summary: "Disallow control flow nested beyond the canonical depth",
        explanation: "Deeply nested control flow obscures the conditions governing each operation. Prefer guards, early exits, or an extracted operation once a function exceeds the standard nesting depth.",
        example: {
            reported: r#"
function acceptsMail(isActive: boolean, hasEmail: boolean, isSubscribed: boolean): boolean {
    if (isActive) {
        if (hasEmail) {
            if (isSubscribed) {
                return true;
            }
        }
    }

    return false;
}
"#,
            accepted: r#"
function acceptsMail(isActive: boolean, hasEmail: boolean, isSubscribed: boolean): boolean {
    if (!isActive || !hasEmail || !isSubscribed) {
        return false;
    }

    return true;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check excessive-nesting.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

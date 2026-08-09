use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require reasons for diagnostic suppressions.
    pub REQUIRE_DIAGNOSTIC_REASON {
        id: "require-diagnostic-reason",
        summary: "Require reasons for diagnostic suppressions",
        explanation: r#"
Every `@allow` and `@expect` must explain why the diagnostic is intentionally suppressed. A reason
preserves the local design decision and makes obsolete controls recognizable during review.
"#,
        example: {
            reported: r#"
@allow("constant-condition")
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
            accepted: r#"
@allow("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check require-diagnostic-reason.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

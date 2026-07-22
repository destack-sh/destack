use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer isMultipleOf over an equivalent remainder comparison.
    pub MANUAL_IS_MULTIPLE_OF {
        id: "manual-is-multiple-of",
        summary: "Prefer isMultipleOf over an equivalent remainder comparison",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-is-multiple-of.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

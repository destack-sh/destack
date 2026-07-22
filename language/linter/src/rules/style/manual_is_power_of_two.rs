use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer isPowerOfTwo over an equivalent bitwise test.
    pub MANUAL_IS_POWER_OF_TWO {
        id: "manual-is-power-of-two",
        summary: "Prefer isPowerOfTwo over an equivalent bitwise test",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-is-power-of-two.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

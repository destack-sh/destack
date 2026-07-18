use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when a large represented value is passed by value.
    pub LARGE_PASS_BY_VALUE {
        id: "large-pass-by-value",
        code: "LP052",
        description: "Warn when a large represented value is passed by value",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-pass-by-value.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when a large represented value is returned by value.
    pub LARGE_RETURN_BY_VALUE {
        id: "large-return-by-value",
        description: "Warn when a large represented value is returned by value",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-return-by-value.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

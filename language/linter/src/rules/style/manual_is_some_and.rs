use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer isSomeAnd over mapping to a defaulted predicate.
    pub MANUAL_IS_SOME_AND {
        id: "manual-is-some-and",
        summary: "Prefer isSomeAnd over mapping to a defaulted predicate",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-is-some-and.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

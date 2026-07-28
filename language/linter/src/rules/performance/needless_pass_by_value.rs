use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow passing parameters by value when they are never consumed or mutated.
    pub NEEDLESS_PASS_BY_VALUE {
        id: "needless-pass-by-value",
        summary: "Disallow passing parameters by value when they are never consumed or mutated",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check needless-pass-by-value.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

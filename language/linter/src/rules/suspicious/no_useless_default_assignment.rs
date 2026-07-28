use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow defaults that cannot be selected by the checked input type.
    pub NO_USELESS_DEFAULT_ASSIGNMENT {
        id: "no-useless-default-assignment",
        summary: "Disallow defaults that cannot be selected by the checked input type",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-useless-default-assignment.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

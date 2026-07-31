use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow branches that only return boolean literals.
    pub NO_NEEDLESS_BOOLEAN_BRANCH {
        id: "no-needless-boolean-branch",
        summary: "Disallow branches that only return boolean literals",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-needless-boolean-branch.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

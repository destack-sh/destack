use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow borrowing small Copy values where copying is cheaper.
    pub NEEDLESS_BORROW_OF_COPY {
        id: "needless-borrow-of-copy",
        summary: "Disallow borrowing small Copy values where copying is cheaper",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check needless-borrow-of-copy.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

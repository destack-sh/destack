use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow closures that only forward their arguments to another callable.
    pub REDUNDANT_CLOSURE {
        id: "redundant-closure",
        description: "Disallow closures that only forward their arguments to another callable",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check redundant-closure.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

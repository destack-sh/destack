use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow empty static initialization blocks.
    pub NO_EMPTY_STATIC_BLOCK {
        id: "no-empty-static-block",
        code: "LU015",
        description: "Disallow empty static initialization blocks",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-empty-static-block.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

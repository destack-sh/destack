use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow async wrappers around a single awaited expression.
    pub NO_REDUNDANT_ASYNC_BLOCK {
        id: "no-redundant-async-block",
        summary: "Disallow async wrappers around a single awaited expression",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-redundant-async-block.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

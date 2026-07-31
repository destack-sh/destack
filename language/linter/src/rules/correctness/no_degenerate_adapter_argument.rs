use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow constant arguments that make an operation a no-op or certain failure.
    pub NO_DEGENERATE_ADAPTER_ARGUMENT {
        id: "no-degenerate-adapter-argument",
        summary: "Disallow constant arguments that make an operation a no-op or certain failure",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-degenerate-adapter-argument.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

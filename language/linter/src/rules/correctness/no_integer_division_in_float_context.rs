use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow integer division whose result immediately becomes a float.
    pub NO_INTEGER_DIVISION_IN_FLOAT_CONTEXT {
        id: "no-integer-division-in-float-context",
        summary: "Disallow integer division whose result immediately becomes a float",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-integer-division-in-float-context.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

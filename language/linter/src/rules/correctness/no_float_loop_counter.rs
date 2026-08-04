use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow float loop counters accumulated by repeated addition.
    pub NO_FLOAT_LOOP_COUNTER {
        id: "no-float-loop-counter",
        summary: "Disallow float loop counters accumulated by repeated addition",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-float-loop-counter.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

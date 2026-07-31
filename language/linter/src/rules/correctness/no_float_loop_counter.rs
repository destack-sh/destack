use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow float loop counters accumulated by repeated addition.
    pub NO_FLOAT_LOOP_COUNTER {
        id: "no-float-loop-counter",
        summary: "Disallow float loop counters accumulated by repeated addition",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check no-float-loop-counter.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

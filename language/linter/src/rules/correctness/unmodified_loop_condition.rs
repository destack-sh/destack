use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow loops whose condition dependencies never change.
    pub UNMODIFIED_LOOP_CONDITION {
        id: "unmodified-loop-condition",
        summary: "Disallow loops whose condition dependencies never change",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check unmodified-loop-condition.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow loops whose condition dependencies never change.
    pub UNMODIFIED_LOOP_CONDITION {
        id: "unmodified-loop-condition",
        description: "Disallow loops whose condition dependencies never change",
        category: Correctness,
        level: Error,
        fixable: Never,
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

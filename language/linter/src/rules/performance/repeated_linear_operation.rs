use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when a linear operation is nested in repeated execution.
    pub REPEATED_LINEAR_OPERATION {
        id: "repeated-linear-operation",
        description: "Warn when a linear operation is nested in repeated execution",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check repeated-linear-operation.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

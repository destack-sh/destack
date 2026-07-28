use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer faster floating-point operations when their precision is sufficient.
    pub SUBOPTIMAL_FLOAT_OPERATION {
        id: "suboptimal-float-operation",
        summary: "Prefer faster floating-point operations when their precision is sufficient",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check suboptimal-float-operation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

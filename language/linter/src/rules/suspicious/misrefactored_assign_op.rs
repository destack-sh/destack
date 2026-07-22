use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow compound assignments that repeat the assigned place.
    pub MISREFACTORED_ASSIGN_OP {
        id: "misrefactored-assign-op",
        summary: "Disallow compound assignments that repeat the assigned place",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check misrefactored-assign-op.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

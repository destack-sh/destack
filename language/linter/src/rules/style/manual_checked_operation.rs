use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer checked arithmetic over equivalent manual bounds checks.
    pub MANUAL_CHECKED_OPERATION {
        id: "manual-checked-operation",
        summary: "Prefer checked arithmetic over equivalent manual bounds checks",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-checked-operation.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

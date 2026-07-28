use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer saturating arithmetic over equivalent manual bounds logic.
    pub MANUAL_SATURATING_ARITHMETIC {
        id: "manual-saturating-arithmetic",
        summary: "Prefer saturating arithmetic over equivalent manual bounds logic",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-saturating-arithmetic.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

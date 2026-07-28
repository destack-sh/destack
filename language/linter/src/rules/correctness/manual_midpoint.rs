use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer midpoint operations that cannot overflow intermediate arithmetic.
    pub MANUAL_MIDPOINT {
        id: "manual-midpoint",
        summary: "Prefer midpoint operations that cannot overflow intermediate arithmetic",
        category: Correctness,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-midpoint.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

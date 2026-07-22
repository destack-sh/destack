use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Disallow equivalent generic instances that duplicate generated program work.
    pub DUPLICATE_MONOMORPHIZATION {
        id: "duplicate-monomorphization",
        summary: "Disallow equivalent generic instances that duplicate generated program work",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check duplicate-monomorphization.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

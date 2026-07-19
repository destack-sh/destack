use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirProgram};

declare_lint! {
    /// Warn when equivalent generic instances duplicate generated program work.
    pub DUPLICATE_MONOMORPHIZATION {
        id: "duplicate-monomorphization",
        description: "Warn when equivalent generic instances duplicate generated program work",
        category: Performance,
        level: Warning,
        fixable: Never,
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

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow direct == comparison of floats.
    pub NO_FLOATING_POINT_EQUALITY {
        id: "no-floating-point-equality",
        description: "Disallow direct == comparison of floats",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-floating-point-equality.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

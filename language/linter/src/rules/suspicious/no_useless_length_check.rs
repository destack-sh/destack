use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow length checks duplicated by the guarded operation.
    pub NO_USELESS_LENGTH_CHECK {
        id: "no-useless-length-check",
        description: "Disallow length checks duplicated by the guarded operation",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-length-check.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

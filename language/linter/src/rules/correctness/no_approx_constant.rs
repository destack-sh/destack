use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow numeric literals that approximate well-known constants.
    pub NO_APPROX_CONSTANT {
        id: "no-approx-constant",
        description: "Disallow numeric literals that approximate well-known constants",
        category: Correctness,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-approx-constant.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

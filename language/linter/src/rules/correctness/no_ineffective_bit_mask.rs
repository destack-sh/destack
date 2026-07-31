use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow bit masks that make their comparison constant.
    pub NO_INEFFECTIVE_BIT_MASK {
        id: "no-ineffective-bit-mask",
        summary: "Disallow bit masks that make their comparison constant",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-ineffective-bit-mask.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

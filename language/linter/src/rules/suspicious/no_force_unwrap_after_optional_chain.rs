use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow force unwrapping an optional chain.
    pub NO_FORCE_UNWRAP_AFTER_OPTIONAL_CHAIN {
        id: "no-force-unwrap-after-optional-chain",
        summary: "Disallow force unwrapping an optional chain",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-force-unwrap-after-optional-chain.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

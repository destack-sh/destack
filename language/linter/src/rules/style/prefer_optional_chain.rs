use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer optional chaining to equivalent guarded property access.
    pub PREFER_OPTIONAL_CHAIN {
        id: "prefer-optional-chain",
        summary: "Prefer optional chaining to equivalent guarded property access",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-optional-chain.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

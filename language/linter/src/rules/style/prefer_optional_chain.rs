use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer optional chaining to equivalent guarded property access.
    pub PREFER_OPTIONAL_CHAIN {
        id: "prefer-optional-chain",
        code: "LY047",
        description: "Prefer optional chaining to equivalent guarded property access",
        category: Style,
        level: Warning,
        fixable: Always,
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

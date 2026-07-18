use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require explicit types at exported and public API boundaries.
    pub EXPLICIT_PUBLIC_TYPES {
        id: "explicit-public-types",
        code: "LY011",
        description: "Require explicit types at exported and public API boundaries",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check explicit-public-types.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

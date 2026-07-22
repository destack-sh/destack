use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer nullish coalescing when only nullish values select the fallback.
    pub PREFER_NULLISH_COALESCING {
        id: "prefer-nullish-coalescing",
        summary: "Prefer nullish coalescing when only nullish values select the fallback",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-nullish-coalescing.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

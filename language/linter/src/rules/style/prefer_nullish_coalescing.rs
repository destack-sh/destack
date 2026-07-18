use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer nullish coalescing when only nullish values select the fallback.
    pub PREFER_NULLISH_COALESCING {
        id: "prefer-nullish-coalescing",
        code: "LY046",
        description: "Prefer nullish coalescing when only nullish values select the fallback",
        category: Style,
        level: Warning,
        fixable: Always,
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

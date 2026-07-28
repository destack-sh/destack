use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer filterMap over equivalent filter and map operations.
    pub MANUAL_FILTER_MAP {
        id: "manual-filter-map",
        summary: "Prefer filterMap over equivalent filter and map operations",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-filter-map.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

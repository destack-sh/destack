use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer clamp over equivalent minimum and maximum operations.
    pub MANUAL_CLAMP {
        id: "manual-clamp",
        summary: "Prefer clamp over equivalent minimum and maximum operations",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-clamp.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

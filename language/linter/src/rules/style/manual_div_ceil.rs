use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer ceiling-division operations over equivalent manual arithmetic.
    pub MANUAL_DIV_CEIL {
        id: "manual-div-ceil",
        summary: "Prefer ceiling-division operations over equivalent manual arithmetic",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-div-ceil.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

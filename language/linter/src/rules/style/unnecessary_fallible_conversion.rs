use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer infallible operations over handling impossible failures.
    pub UNNECESSARY_FALLIBLE_CONVERSION {
        id: "unnecessary-fallible-conversion",
        summary: "Prefer infallible operations over handling impossible failures",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check unnecessary-fallible-conversion.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer unwrapOrDefault over equivalent pattern matching.
    pub MANUAL_UNWRAP_OR_DEFAULT {
        id: "manual-unwrap-or-default",
        summary: "Prefer unwrapOrDefault over equivalent pattern matching",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-unwrap-or-default.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

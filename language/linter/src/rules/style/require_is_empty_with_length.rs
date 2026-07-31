use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require isEmpty alongside a length accessor.
    pub REQUIRE_IS_EMPTY_WITH_LENGTH {
        id: "require-is-empty-with-length",
        summary: "Require isEmpty alongside a length accessor",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check require-is-empty-with-length.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

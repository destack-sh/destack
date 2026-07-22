use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer fragment shorthand when no fragment properties are present.
    pub PREFER_FRAGMENT_SHORTHAND {
        id: "prefer-fragment-shorthand",
        summary: "Prefer fragment shorthand when no fragment properties are present",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-fragment-shorthand.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

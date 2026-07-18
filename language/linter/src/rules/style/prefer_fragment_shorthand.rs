use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer fragment shorthand when no fragment properties are present.
    pub PREFER_FRAGMENT_SHORTHAND {
        id: "prefer-fragment-shorthand",
        code: "LY039",
        description: "Prefer fragment shorthand when no fragment properties are present",
        category: Style,
        level: Warning,
        fixable: Always,
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

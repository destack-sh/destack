use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow nested ternary expressions.
    pub NO_NESTED_TERNARY {
        id: "no-nested-ternary",
        description: "Disallow nested ternary expressions",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-nested-ternary.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

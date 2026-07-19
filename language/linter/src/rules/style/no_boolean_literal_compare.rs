use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow comparing to boolean literals.
    pub NO_BOOLEAN_LITERAL_COMPARE {
        id: "no-boolean-literal-compare",
        description: "Disallow comparing to boolean literals",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-boolean-literal-compare.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

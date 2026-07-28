use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer tuple destructuring over indexed access.
    pub PREFER_TUPLE_DESTRUCTURE {
        id: "prefer-tuple-destructure",
        summary: "Prefer tuple destructuring over indexed access",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-destructure.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer this as the return type when a method only returns its receiver.
    pub PREFER_RETURN_THIS_TYPE {
        id: "prefer-return-this-type",
        summary: "Prefer this as the return type when a method only returns its receiver",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-return-this-type.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

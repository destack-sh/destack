use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer named regex capture groups when captured values are consumed.
    pub PREFER_NAMED_CAPTURE_GROUP {
        id: "prefer-named-capture-group",
        summary: "Prefer named regex capture groups when captured values are consumed",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-named-capture-group.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer nesting or-patterns inside their shared constructor.
    pub PREFER_NESTED_OR_PATTERN {
        id: "prefer-nested-or-pattern",
        summary: "Prefer nesting or-patterns inside their shared constructor",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-nested-or-pattern.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer expression initialization over staged assignment.
    pub PREFER_EXPRESSION_INITIALIZATION {
        id: "prefer-expression-initialization",
        summary: "Prefer expression initialization over staged assignment",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check prefer-expression-initialization.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

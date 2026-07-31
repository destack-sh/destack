use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow display implementations omitting fields.
    pub INCOMPLETE_DISPLAY_IMPLEMENTATION {
        id: "incomplete-display-implementation",
        summary: "Disallow display implementations omitting fields",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check incomplete-display-implementation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

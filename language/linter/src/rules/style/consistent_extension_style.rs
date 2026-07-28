use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require the canonical extension declaration form for its visibility.
    pub CONSISTENT_EXTENSION_STYLE {
        id: "consistent-extension-style",
        summary: "Require the canonical extension declaration form for its visibility",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check consistent-extension-style.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

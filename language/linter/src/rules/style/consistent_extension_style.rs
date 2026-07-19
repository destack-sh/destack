use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require the canonical extension declaration form for its visibility.
    pub CONSISTENT_EXTENSION_STYLE {
        id: "consistent-extension-style",
        description: "Require the canonical extension declaration form for its visibility",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check consistent-extension-style.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

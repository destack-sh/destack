use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer replaceAll when every string occurrence is replaced.
    pub PREFER_STRING_REPLACE_ALL {
        id: "prefer-string-replace-all",
        summary: "Prefer replaceAll when every string occurrence is replaced",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-string-replace-all.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

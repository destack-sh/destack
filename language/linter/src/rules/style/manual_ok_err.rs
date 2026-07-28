use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer Result projection methods over equivalent pattern matching.
    pub MANUAL_OK_ERR {
        id: "manual-ok-err",
        summary: "Prefer Result projection methods over equivalent pattern matching",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-ok-err.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

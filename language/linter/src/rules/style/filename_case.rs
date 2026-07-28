use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Enforce filename case style.
    pub FILENAME_CASE {
        id: "filename-case",
        summary: "Enforce filename case style",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check filename-case.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

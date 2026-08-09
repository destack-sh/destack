use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow doc links that resolve to nothing.
    pub UNRESOLVED_DOC_LINK {
        id: "unresolved-doc-link",
        summary: "Disallow doc links that resolve to nothing",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirProgram(check),
    }
}

/// Check unresolved-doc-link.
fn check(_program: &DirProgram<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

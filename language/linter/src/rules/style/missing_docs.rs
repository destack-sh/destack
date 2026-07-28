use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require documentation for public declarations.
    pub MISSING_DOCS {
        id: "missing-docs",
        summary: "Require documentation for public declarations",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check missing-docs.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

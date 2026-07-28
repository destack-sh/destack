use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require a safety rationale for every unsafe declaration and expression.
    pub UNDOCUMENTED_UNSAFE {
        id: "undocumented-unsafe",
        summary: "Require a safety rationale for every unsafe declaration and expression",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check undocumented-unsafe.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

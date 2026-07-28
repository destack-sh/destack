use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require overload signatures for one declaration to be adjacent.
    pub ADJACENT_OVERLOAD_SIGNATURES {
        id: "adjacent-overload-signatures",
        summary: "Require overload signatures for one declaration to be adjacent",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check adjacent-overload-signatures.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

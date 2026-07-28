use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Combine overloads that differ only by one representable parameter shape.
    pub UNIFIED_SIGNATURES {
        id: "unified-signatures",
        summary: "Combine overloads that differ only by one representable parameter shape",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check unified-signatures.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

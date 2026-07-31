use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow dropping values without destructors.
    pub NO_DROP_OF_TRIVIAL_VALUE {
        id: "no-drop-of-trivial-value",
        summary: "Disallow dropping values without destructors",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check no-drop-of-trivial-value.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

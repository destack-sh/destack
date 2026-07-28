use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require getter and setter pairs to be adjacent.
    pub GROUPED_ACCESSOR_PAIRS {
        id: "grouped-accessor-pairs",
        summary: "Require getter and setter pairs to be adjacent",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check grouped-accessor-pairs.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require getter and setter pairs to be adjacent.
    pub GROUPED_ACCESSOR_PAIRS {
        id: "grouped-accessor-pairs",
        description: "Require getter and setter pairs to be adjacent",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check grouped-accessor-pairs.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

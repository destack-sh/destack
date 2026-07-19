use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer positive conditions when both branches are present.
    pub NO_NEGATED_CONDITION {
        id: "no-negated-condition",
        description: "Prefer positive conditions when both branches are present",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-negated-condition.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

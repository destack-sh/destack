use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Suggest merging nested if statements.
    pub NO_COLLAPSIBLE_IF {
        id: "no-collapsible-if",
        description: "Suggest merging nested if statements",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-collapsible-if.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

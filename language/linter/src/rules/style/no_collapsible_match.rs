use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Merge nested matches whose inner pattern fits the outer arm.
    pub NO_COLLAPSIBLE_MATCH {
        id: "no-collapsible-match",
        summary: "Merge nested matches whose inner pattern fits the outer arm",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-collapsible-match.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

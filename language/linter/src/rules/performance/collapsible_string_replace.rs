use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Combine chained replacements sharing one replacement value.
    pub COLLAPSIBLE_STRING_REPLACE {
        id: "collapsible-string-replace",
        summary: "Combine chained replacements sharing one replacement value",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check collapsible-string-replace.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

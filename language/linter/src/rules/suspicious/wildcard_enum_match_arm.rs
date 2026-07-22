use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow wildcard arms that hide later enum variants.
    pub WILDCARD_ENUM_MATCH_ARM {
        id: "wildcard-enum-match-arm",
        summary: "Disallow wildcard arms that hide later enum variants",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check wildcard-enum-match-arm.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer direct result predicates over pattern matching used only as a test.
    pub REDUNDANT_PATTERN_MATCHING {
        id: "redundant-pattern-matching",
        summary: "Prefer direct result predicates over pattern matching used only as a test",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check redundant-pattern-matching.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

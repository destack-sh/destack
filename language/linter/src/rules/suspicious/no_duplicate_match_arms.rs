use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow match arms with identical checked bodies.
    pub NO_DUPLICATE_MATCH_ARMS {
        id: "no-duplicate-match-arms",
        summary: "Disallow match arms with identical checked bodies",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-duplicate-match-arms.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

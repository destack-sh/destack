use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Warn on match arms with identical checked bodies.
    pub NO_DUPLICATE_MATCH_ARMS {
        id: "no-duplicate-match-arms",
        description: "Warn on match arms with identical checked bodies",
        category: Suspicious,
        level: Warning,
        fixable: Never,
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

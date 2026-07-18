use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow misleading regex character classes.
    pub NO_MISLEADING_CHARACTER_CLASS {
        id: "no-misleading-character-class",
        code: "LU022",
        description: "Disallow misleading regex character classes",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-misleading-character-class.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow empty block statements.
    pub NO_EMPTY {
        id: "no-empty",
        description: "Disallow empty block statements",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-empty.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

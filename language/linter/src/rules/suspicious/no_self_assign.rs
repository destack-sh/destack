use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow self-assignment.
    pub NO_SELF_ASSIGN {
        id: "no-self-assign",
        description: "Disallow self-assignment",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-self-assign.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

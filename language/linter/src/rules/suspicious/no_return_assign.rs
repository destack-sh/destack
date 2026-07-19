use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow assignment within an explicit or implicit return value.
    pub NO_RETURN_ASSIGN {
        id: "no-return-assign",
        description: "Disallow assignment within an explicit or implicit return value",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-return-assign.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require const for bindings never reassigned after initialization.
    pub PREFER_CONST {
        id: "prefer-const",
        description: "Require const for bindings never reassigned after initialization",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-const.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

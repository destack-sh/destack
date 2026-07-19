use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require readonly for fields never mutated after initialization.
    pub PREFER_READONLY {
        id: "prefer-readonly",
        description: "Require readonly for fields never mutated after initialization",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-readonly.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

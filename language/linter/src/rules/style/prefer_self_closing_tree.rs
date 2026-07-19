use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer self-closing tree tags.
    pub PREFER_SELF_CLOSING_TREE {
        id: "prefer-self-closing-tree",
        description: "Prefer self-closing tree tags",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-self-closing-tree.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

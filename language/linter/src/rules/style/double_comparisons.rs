use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer one comparison over an equivalent pair of comparisons.
    pub DOUBLE_COMPARISONS {
        id: "double-comparisons",
        summary: "Prefer one comparison over an equivalent pair of comparisons",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check double-comparisons.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

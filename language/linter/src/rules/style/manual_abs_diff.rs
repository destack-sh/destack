use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer absolute-difference operations over equivalent branching arithmetic.
    pub MANUAL_ABS_DIFF {
        id: "manual-abs-diff",
        summary: "Prefer absolute-difference operations over equivalent branching arithmetic",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-abs-diff.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

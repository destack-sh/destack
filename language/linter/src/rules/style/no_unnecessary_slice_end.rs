use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow slice end arguments equal to the source length.
    pub NO_UNNECESSARY_SLICE_END {
        id: "no-unnecessary-slice-end",
        summary: "Disallow slice end arguments equal to the source length",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-unnecessary-slice-end.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

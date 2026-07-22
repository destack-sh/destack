use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow an explicit default array flattening depth.
    pub NO_UNNECESSARY_ARRAY_FLAT_DEPTH {
        id: "no-unnecessary-array-flat-depth",
        summary: "Disallow an explicit default array flattening depth",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-unnecessary-array-flat-depth.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

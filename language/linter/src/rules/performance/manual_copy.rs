use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Replace element-by-element copy loops with a bulk copy operation.
    pub MANUAL_COPY {
        id: "manual-copy",
        summary: "Replace element-by-element copy loops with a bulk copy operation",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check manual-copy.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

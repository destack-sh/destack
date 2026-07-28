use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow allocating operations on repeated paths.
    pub ALLOCATION_IN_LOOP {
        id: "allocation-in-loop",
        summary: "Disallow allocating operations on repeated paths",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check allocation-in-loop.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

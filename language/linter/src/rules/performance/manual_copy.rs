use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Replace element-by-element copy loops with a bulk copy operation.
    pub MANUAL_COPY {
        id: "manual-copy",
        description: "Replace element-by-element copy loops with a bulk copy operation",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check manual-copy.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

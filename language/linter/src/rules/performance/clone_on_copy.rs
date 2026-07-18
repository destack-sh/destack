use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow cloning values whose checked type is Copy.
    pub CLONE_ON_COPY {
        id: "clone-on-copy",
        code: "LP062",
        description: "Disallow cloning values whose checked type is Copy",
        category: Performance,
        level: Warning,
        fixable: Always,
        check: MirModule(check),
    }
}

/// Check clone-on-copy.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

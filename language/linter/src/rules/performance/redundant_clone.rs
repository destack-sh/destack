use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow clones proven unnecessary by ownership and liveness.
    pub REDUNDANT_CLONE {
        id: "redundant-clone",
        summary: "Disallow clones proven unnecessary by ownership and liveness",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check redundant-clone.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

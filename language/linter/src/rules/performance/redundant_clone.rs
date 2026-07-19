use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow clones proven unnecessary by ownership and liveness.
    pub REDUNDANT_CLONE {
        id: "redundant-clone",
        description: "Disallow clones proven unnecessary by ownership and liveness",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check redundant-clone.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

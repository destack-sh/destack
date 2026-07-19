use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow loops that must exit during their first iteration.
    pub LOOP_SINGLE_ITERATION {
        id: "loop-single-iteration",
        description: "Disallow loops that must exit during their first iteration",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check loop-single-iteration.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow updates derived from values read before suspension.
    pub STALE_UPDATE_ACROSS_SUSPENSION {
        id: "stale-update-across-suspension",
        code: "LC055",
        description: "Disallow updates derived from values read before suspension",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check stale-update-across-suspension.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

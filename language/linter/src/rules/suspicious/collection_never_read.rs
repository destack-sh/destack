use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow collections written but never observed.
    pub COLLECTION_NEVER_READ {
        id: "collection-never-read",
        summary: "Disallow collections written but never observed",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check collection-never-read.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

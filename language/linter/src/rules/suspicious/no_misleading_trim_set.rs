use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow multi-character trim sets where a prefix strip was intended.
    pub NO_MISLEADING_TRIM_SET {
        id: "no-misleading-trim-set",
        summary: "Disallow multi-character trim sets where a prefix strip was intended",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-misleading-trim-set.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

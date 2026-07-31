use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow world-writable and decimal file permission literals.
    pub NO_PERMISSIVE_FILE_PERMISSION {
        id: "no-permissive-file-permission",
        summary: "Disallow world-writable and decimal file permission literals",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-permissive-file-permission.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

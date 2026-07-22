use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow credentials and secret material embedded in source values.
    pub HARDCODED_SECRET {
        id: "hardcoded-secret",
        summary: "Disallow credentials and secret material embedded in source values",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check hardcoded-secret.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

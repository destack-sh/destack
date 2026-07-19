use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow credentials and secret material embedded in source values.
    pub HARDCODED_SECRET {
        id: "hardcoded-secret",
        description: "Disallow credentials and secret material embedded in source values",
        category: Security,
        level: Error,
        fixable: Never,
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

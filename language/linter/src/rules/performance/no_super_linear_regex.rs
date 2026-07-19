use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow regex with potential catastrophic backtracking.
    pub NO_SUPER_LINEAR_REGEX {
        id: "no-super-linear-regex",
        description: "Disallow regex with potential catastrophic backtracking",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-super-linear-regex.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

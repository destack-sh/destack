use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow regex with potential catastrophic backtracking.
    pub NO_SUPER_LINEAR_REGEX {
        id: "no-super-linear-regex",
        summary: "Disallow regex with potential catastrophic backtracking",
        category: Security,
        level: Warning,
        fixable: None,
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

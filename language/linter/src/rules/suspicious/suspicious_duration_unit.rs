use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow bare numbers where a duration unit is implied.
    pub SUSPICIOUS_DURATION_UNIT {
        id: "suspicious-duration-unit",
        summary: "Disallow bare numbers where a duration unit is implied",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check suspicious-duration-unit.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

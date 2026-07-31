use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow match arms unreachable under the scrutinee's case mapping.
    pub NO_CASE_MISMATCHED_MATCH_ARM {
        id: "no-case-mismatched-match-arm",
        summary: "Disallow match arms unreachable under the scrutinee's case mapping",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-case-mismatched-match-arm.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

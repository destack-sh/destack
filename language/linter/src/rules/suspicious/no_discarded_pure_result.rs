use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Disallow discarding results of calls proven pure.
    pub NO_DISCARDED_PURE_RESULT {
        id: "no-discarded-pure-result",
        summary: "Disallow discarding results of calls proven pure",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check no-discarded-pure-result.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

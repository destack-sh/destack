use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow substantial alpha-equivalent checked code.
    pub NO_DUPLICATE_CODE {
        id: "no-duplicate-code",
        summary: "Disallow substantial alpha-equivalent checked code",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram(check),
    }
}

/// Check no-duplicate-code.
fn check(_program: &DirProgram<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

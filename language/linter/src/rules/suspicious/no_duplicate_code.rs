use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirProgram, Lint, LintResult};

declare_lint! {
    /// Disallow substantial alpha-equivalent checked code.
    pub NO_DUPLICATE_CODE {
        id: "no-duplicate-code",
        description: "Disallow substantial alpha-equivalent checked code",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check no-duplicate-code.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

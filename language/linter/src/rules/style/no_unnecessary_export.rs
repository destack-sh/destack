use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow exports never imported by the target program.
    pub NO_UNNECESSARY_EXPORT {
        id: "no-unnecessary-export",
        summary: "Disallow exports never imported by the target program",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirProgram(check),
    }
}

/// Check no-unnecessary-export.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

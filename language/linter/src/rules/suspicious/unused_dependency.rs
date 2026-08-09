use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow declared dependencies unused by the target program.
    pub UNUSED_DEPENDENCY {
        id: "unused-dependency",
        summary: "Disallow declared dependencies unused by the target program",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram(check),
    }
}

/// Check unused-dependency.
fn check(_program: &DirProgram<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow cyclic dependencies between source modules.
    pub CYCLIC_DEPENDENCY {
        id: "cyclic-dependency",
        summary: "Disallow cyclic dependencies between source modules",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram(check),
    }
}

/// Check cyclic-dependency.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

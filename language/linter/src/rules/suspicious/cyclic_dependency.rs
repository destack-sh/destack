use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirProgram, Lint, LintResult};

declare_lint! {
    /// Disallow cyclic dependencies between source modules.
    pub CYCLIC_DEPENDENCY {
        id: "cyclic-dependency",
        description: "Disallow cyclic dependencies between source modules",
        category: Suspicious,
        level: Warning,
        fixable: Never,
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

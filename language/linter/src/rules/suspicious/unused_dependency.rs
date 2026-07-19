use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirProgram, Lint, LintResult};

declare_lint! {
    /// Warn on declared dependencies unused by the target program.
    pub UNUSED_DEPENDENCY {
        id: "unused-dependency",
        description: "Warn on declared dependencies unused by the target program",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check unused-dependency.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

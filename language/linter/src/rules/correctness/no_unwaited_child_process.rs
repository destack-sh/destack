use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow spawned processes never awaited on any path.
    pub NO_UNWAITED_CHILD_PROCESS {
        id: "no-unwaited-child-process",
        summary: "Disallow spawned processes never awaited on any path",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check no-unwaited-child-process.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

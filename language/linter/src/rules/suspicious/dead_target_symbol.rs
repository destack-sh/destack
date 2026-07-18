use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirProgram};

declare_lint! {
    /// Warn on target symbols unreachable from program roots.
    pub DEAD_TARGET_SYMBOL {
        id: "dead-target-symbol",
        code: "LU061",
        description: "Warn on target symbols unreachable from program roots",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check dead-target-symbol.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

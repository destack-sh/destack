use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Disallow code unreachable from every target program root.
    pub DEAD_CODE {
        id: "dead-code",
        summary: "Disallow code unreachable from every target program root",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check dead-code.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

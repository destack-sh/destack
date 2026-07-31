use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Require comptime for functions whose bodies qualify.
    pub MISSING_COMPTIME {
        id: "missing-comptime",
        summary: "Require comptime for functions whose bodies qualify",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: MirProgram(check),
    }
}

/// Check missing-comptime.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Disallow values carrying a source taint from reaching matching sinks.
    pub NO_TAINTED_SINK {
        id: "no-tainted-sink",
        summary: "Disallow values carrying a source taint from reaching matching sinks",
        category: Security,
        level: Error,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check no-tainted-sink.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow resolved blocking operations in asynchronous execution.
    pub BLOCKING_CALL_IN_ASYNC {
        id: "blocking-call-in-async",
        summary: "Disallow resolved blocking operations in asynchronous execution",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check blocking-call-in-async.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow resolved blocking operations in asynchronous execution.
    pub BLOCKING_CALL_IN_ASYNC {
        id: "blocking-call-in-async",
        code: "LC054",
        description: "Disallow resolved blocking operations in asynchronous execution",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check blocking-call-in-async.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}

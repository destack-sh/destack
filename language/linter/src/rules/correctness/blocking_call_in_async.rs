use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

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
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

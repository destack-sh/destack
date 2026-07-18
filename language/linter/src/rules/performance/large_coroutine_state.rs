use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when values retained across suspension create a large coroutine.
    pub LARGE_COROUTINE_STATE {
        id: "large-coroutine-state",
        code: "LP056",
        description: "Warn when values retained across suspension create a large coroutine",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-coroutine-state.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

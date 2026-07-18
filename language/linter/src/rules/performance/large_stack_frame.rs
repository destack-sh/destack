use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when one function requires excessive stack storage.
    pub LARGE_STACK_FRAME {
        id: "large-stack-frame",
        code: "LP055",
        description: "Warn when one function requires excessive stack storage",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-stack-frame.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

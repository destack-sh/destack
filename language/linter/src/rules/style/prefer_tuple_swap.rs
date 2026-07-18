use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer tuple assignment for swaps.
    pub PREFER_TUPLE_SWAP {
        id: "prefer-tuple-swap",
        code: "LY060",
        description: "Prefer tuple assignment for swaps",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-swap.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

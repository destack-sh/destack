use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when one variant disproportionately enlarges an inline union.
    pub LARGE_VARIANT {
        id: "large-variant",
        code: "LP063",
        description: "Warn when one variant disproportionately enlarges an inline union",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-variant.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

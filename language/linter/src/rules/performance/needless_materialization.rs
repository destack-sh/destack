use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow collections built only for immediate consumption.
    pub NEEDLESS_MATERIALIZATION {
        id: "needless-materialization",
        code: "LP061",
        description: "Disallow collections built only for immediate consumption",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check needless-materialization.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

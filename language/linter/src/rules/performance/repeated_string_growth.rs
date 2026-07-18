use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when repeated string growth causes cumulative copying.
    pub REPEATED_STRING_GROWTH {
        id: "repeated-string-growth",
        code: "LP060",
        description: "Warn when repeated string growth causes cumulative copying",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check repeated-string-growth.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

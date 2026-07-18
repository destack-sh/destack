use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when significant destruction is extended by a scrutinee.
    pub SIGNIFICANT_DROP_IN_SCRUTINEE {
        id: "significant-drop-in-scrutinee",
        code: "LU051",
        description: "Warn when significant destruction is extended by a scrutinee",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check significant-drop-in-scrutinee.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

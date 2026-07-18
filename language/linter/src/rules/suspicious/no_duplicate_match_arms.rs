use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Warn on match arms with identical checked bodies.
    pub NO_DUPLICATE_MATCH_ARMS {
        id: "no-duplicate-match-arms",
        code: "LU011",
        description: "Warn on match arms with identical checked bodies",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-duplicate-match-arms.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

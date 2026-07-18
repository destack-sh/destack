use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow misleading regex character classes.
    pub NO_MISLEADING_CHARACTER_CLASS {
        id: "no-misleading-character-class",
        code: "LU022",
        description: "Disallow misleading regex character classes",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-misleading-character-class.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

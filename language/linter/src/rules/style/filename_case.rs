use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Enforce filename case style.
    pub FILENAME_CASE {
        id: "filename-case",
        code: "LY012",
        description: "Enforce filename case style",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check filename-case.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

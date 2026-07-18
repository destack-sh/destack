use crate::rules::declare_lint;
use crate::{DirProgramContext, LinterError};

declare_lint! {
    /// Warn on exports unreachable from every target consumer.
    pub UNREACHABLE_EXPORT {
        id: "unreachable-export",
        code: "LU062",
        description: "Warn on exports unreachable from every target consumer",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check unreachable-export.
fn check(context: DirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

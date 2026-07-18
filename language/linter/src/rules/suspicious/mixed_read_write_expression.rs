use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow reading and mutating the same place within one larger expression.
    pub MIXED_READ_WRITE_EXPRESSION {
        id: "mixed-read-write-expression",
        code: "LU052",
        description: "Disallow reading and mutating the same place within one larger expression",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check mixed-read-write-expression.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

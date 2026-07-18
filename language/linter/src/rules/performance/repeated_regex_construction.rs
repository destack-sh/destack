use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when the same regular expression is built repeatedly.
    pub REPEATED_REGEX_CONSTRUCTION {
        id: "repeated-regex-construction",
        code: "LP059",
        description: "Warn when the same regular expression is built repeatedly",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check repeated-regex-construction.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

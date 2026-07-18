use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Enforce default parameters to be last.
    pub DEFAULT_PARAM_LAST {
        id: "default-param-last",
        code: "LY008",
        description: "Enforce default parameters to be last",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check default-param-last.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

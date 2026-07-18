use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow template interpolation in regular strings.
    pub NO_TEMPLATE_CURLY_IN_STRING {
        id: "no-template-curly-in-string",
        code: "LU031",
        description: "Disallow template interpolation in regular strings",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-template-curly-in-string.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow useless regex backreferences.
    pub NO_USELESS_BACKREFERENCE {
        id: "no-useless-backreference",
        code: "LU034",
        description: "Disallow useless regex backreferences",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-backreference.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

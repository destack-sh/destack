use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require getter and setter pairs to be adjacent.
    pub GROUPED_ACCESSOR_PAIRS {
        id: "grouped-accessor-pairs",
        code: "LY013",
        description: "Require getter and setter pairs to be adjacent",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check grouped-accessor-pairs.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

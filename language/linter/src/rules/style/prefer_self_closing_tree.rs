use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer self-closing tree tags.
    pub PREFER_SELF_CLOSING_TREE {
        id: "prefer-self-closing-tree",
        code: "LY053",
        description: "Prefer self-closing tree tags",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-self-closing-tree.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}

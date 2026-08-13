use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer this as the return type when a method only returns its receiver.
    pub PREFER_RETURN_THIS_TYPE {
        id: "prefer-return-this-type",
        summary: "Prefer this as the return type when a method only returns its receiver",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

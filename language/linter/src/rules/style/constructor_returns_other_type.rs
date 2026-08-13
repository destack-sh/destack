use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow constructor-shaped statics returning a different type.
    pub CONSTRUCTOR_RETURNS_OTHER_TYPE {
        id: "constructor-returns-other-type",
        summary: "Disallow constructor-shaped statics returning a different type",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

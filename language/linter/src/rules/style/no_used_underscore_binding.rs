use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow reading bindings named as unused.
    pub NO_USED_UNDERSCORE_BINDING {
        id: "no-used-underscore-binding",
        summary: "Disallow reading bindings named as unused",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

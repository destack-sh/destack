use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer object destructuring over repeated property reads.
    pub PREFER_OBJECT_DESTRUCTURE {
        id: "prefer-object-destructure",
        summary: "Prefer object destructuring over repeated property reads",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}

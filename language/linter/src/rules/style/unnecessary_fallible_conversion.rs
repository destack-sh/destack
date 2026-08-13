use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer infallible operations over handling impossible failures.
    pub UNNECESSARY_FALLIBLE_CONVERSION {
        id: "unnecessary-fallible-conversion",
        summary: "Prefer infallible operations over handling impossible failures",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

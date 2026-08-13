use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow local annotations identical to the inferred type.
    pub NO_INFERRABLE_TYPE_ANNOTATION {
        id: "no-inferrable-type-annotation",
        summary: "Disallow local annotations identical to the inferred type",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}

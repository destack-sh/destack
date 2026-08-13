use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer named regex capture groups when captured values are consumed.
    pub PREFER_NAMED_CAPTURE_GROUP {
        id: "prefer-named-capture-group",
        summary: "Prefer named regex capture groups when captured values are consumed",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

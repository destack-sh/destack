use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer the weakest access form required by a value's uses.
    pub PREFER_WEAKEST_ACCESS {
        id: "prefer-weakest-access",
        summary: "Prefer the weakest access form required by a value's uses",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

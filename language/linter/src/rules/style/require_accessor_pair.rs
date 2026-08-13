use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow setters without a matching getter.
    pub REQUIRE_ACCESSOR_PAIR {
        id: "require-accessor-pair",
        summary: "Disallow setters without a matching getter",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer regex literals for constant patterns and flags.
    pub PREFER_REGEX_LITERALS {
        id: "prefer-regex-literals",
        summary: "Prefer regex literals for constant patterns and flags",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}

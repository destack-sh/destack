use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow member names repeating their owner's name.
    pub REDUNDANT_NAME_PREFIX {
        id: "redundant-name-prefix",
        summary: "Disallow member names repeating their owner's name",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

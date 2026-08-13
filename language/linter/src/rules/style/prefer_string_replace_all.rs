use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer replaceAll when every string occurrence is replaced.
    pub PREFER_STRING_REPLACE_ALL {
        id: "prefer-string-replace-all",
        summary: "Prefer replaceAll when every string occurrence is replaced",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow control characters in regex.
    pub NO_CONTROL_REGEX {
        id: "no-control-regex",
        summary: "Disallow control characters in regex",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

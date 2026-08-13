use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow regex with potential catastrophic backtracking.
    pub NO_SUPER_LINEAR_REGEX {
        id: "no-super-linear-regex",
        summary: "Disallow regex with potential catastrophic backtracking",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow bare numbers where a duration unit is implied.
    pub SUSPICIOUS_DURATION_UNIT {
        id: "suspicious-duration-unit",
        summary: "Disallow bare numbers where a duration unit is implied",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

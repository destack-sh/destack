use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow credentials and secret material embedded in source values.
    pub HARDCODED_SECRET {
        id: "hardcoded-secret",
        summary: "Disallow credentials and secret material embedded in source values",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}

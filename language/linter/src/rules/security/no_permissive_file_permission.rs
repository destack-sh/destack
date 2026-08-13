use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow world-writable and decimal file permission literals.
    pub NO_PERMISSIVE_FILE_PERMISSION {
        id: "no-permissive-file-permission",
        summary: "Disallow world-writable and decimal file permission literals",
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
